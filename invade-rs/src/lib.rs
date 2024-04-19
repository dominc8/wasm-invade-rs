mod static_allocator;

use std::cell::OnceCell;

const WIDTH: usize = 200;
const HEIGHT: usize = 150;
const MULT: usize = 6;
const BUFFER_SIZE: usize = WIDTH * MULT * HEIGHT * MULT;
const MAX_BULLETS: usize = 128;
const MAX_ENEMIES: usize = 16;

#[no_mangle]
static mut BUFFER: [u32; BUFFER_SIZE] = [0; BUFFER_SIZE];

struct Bitmap2D {
    width: u32,
    height: u32,
    bitmap: &'static [u16]
}

/*
 *       OO     
 *      OOOO    
 *      OOOO    
 *  OOOOOOOOOOOO 
 * OOOOOOOOOOOOOO
 * OOOOOOOOOOOOOO
 * OOOOOOOOOOOOOO
 * */
const PLAYER_BITMAP: Bitmap2D = Bitmap2D { width: 14, height: 7,
bitmap: &[
    0b0000001100000000,
    0b0000011110000000,
    0b0000011110000000,
    0b0111111111111000,
    0b1111111111111100,
    0b1111111111111100,
    0b1111111111111100,
] };

/*
 *   O      O  
 *    O    O   
 *   OOOOOOOO  
 *  OO OOOO OO 
 * OOOOOOOOOOOO
 * O OOOOOOOO O
 * O O      O O
 *    OO  OO   
 * */
const ENEMY_BITMAP: Bitmap2D = Bitmap2D { width: 12, height: 8,
bitmap: &[
    0b0010000001000000,
    0b0001000010000000,
    0b0011111111000000,
    0b0110111101100000,
    0b1111111111110000,
    0b1011111111010000,
    0b1010000001010000,
    0b0001100110000000,
] };

struct Player {
    pos: i32,
    color: u32,
    health: i32,
}

#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Background,
    Player,
    Bullet,
    Obstacle,
    Enemy(i8),
}
const DEFAULT_TILE: Tile = Tile::Background;
const ENEMY_SPEED: u8 = 1;

struct Enemy {
    x: u8,
    y: u8,
    health: i8,
}

enum BulletStatus {
    Alive,
    HitPlayer,
    HitEnemy,
    HitObstacle
}

struct Bullet {
    x: u8,
    y: u8,
    speed: i8,
    status: BulletStatus,
}

#[derive(Eq, PartialEq)]
enum GameState {
    StartScreen,
    Playing,
    EndScreen(bool)
}

struct KeyEvent (u32);

impl KeyEvent {
    fn pressed_left(&self) -> bool {
        (self.0 & 1) != 0
    }
    fn pressed_right(&self) -> bool {
        (self.0 & 2) != 0
    }
    fn pressed_space(&self) -> bool {
        (self.0 & 4) != 0
    }
}

struct Game<'a> {
    default_color: u32,
    random_seed: u32,
    game_state: GameState,
    player: Player,
    enemies: static_allocator::SVector<Enemy>,
    bullets: static_allocator::SVector<Bullet>,
    buffer: &'a mut [Tile; WIDTH * HEIGHT],
    seconds_since_start: u32,
    moving_right: bool,
}

static mut GAMECELL: OnceCell<&mut Game> = OnceCell::new();
static mut GAMEBUFFER: [Tile; WIDTH * HEIGHT] = [DEFAULT_TILE; WIDTH * HEIGHT];

fn get_key_event(key_event_flags: u32) -> KeyEvent {
    KeyEvent(key_event_flags)
}

#[no_mangle]
pub unsafe extern fn js_game_init() {
    let game = static_allocator::static_alloc::<Game>();
    game.default_color = 0xFF_FF_FF_FF;
    game.game_state = GameState::StartScreen;
    game.player = Player { pos: (WIDTH as i32)/2, color: 0xFF_00_00_FF, health: 3 };
    game.enemies = static_allocator::SVector::new(MAX_ENEMIES);
    game.bullets = static_allocator::SVector::new(MAX_BULLETS);
    game.moving_right = true;
    game.buffer = &mut GAMEBUFFER;

    game.seconds_since_start = 0;
    let _ = GAMECELL.set(game);
}

#[no_mangle]
pub unsafe extern fn js_game_tick(key_event_flags: u32) {
    if let Some(game) = GAMECELL.get_mut() {
        let key_event = get_key_event(key_event_flags);

        match game.game_state {
            GameState::StartScreen => {
                game.draw_start_screen(&mut BUFFER);
                if key_event.pressed_space() {
                    game.reset_level();
                    game.game_state = GameState::Playing;
                }
            },
            GameState::Playing => {
                game.tick(key_event);
                game.update_buffer();
                game.render(&mut BUFFER);
            },
            GameState::EndScreen(has_won) => {
                game.draw_end_screen(has_won, &mut BUFFER);
                if key_event.pressed_space() {
                    game.game_state = GameState::StartScreen;
                }
            },
            
        }
    }
}

impl Game<'_> {
    fn reset_level(&mut self) {
        self.player.pos = (WIDTH as i32)/2;
        self.player.health = 3;
        self.enemies.reset();
        self.bullets.reset();
        self.moving_right = true;
        self.enemies.push_back(Enemy { x: 20, y: 20, health: 2 });
        self.enemies.push_back(Enemy { x: 60, y: 20, health: 2 });
        self.enemies.push_back(Enemy { x: 40, y: 40, health: 2 });
        self.buffer.fill(Tile::Background);

        self.buffer_fill_obstacles();
    }

    fn buffer_fill_obstacles(&mut self) {
        let y0 = HEIGHT * 5 / 8;
        let width = 4;
        let height = 2;
        let spacing = width * 3;
        let x_step = width + spacing;
        let n_obstacle = WIDTH / x_step;
        let x0_offset = (WIDTH - (n_obstacle * x_step - spacing)) / 2;
        for y in y0..y0+height {
            let x0 = y * WIDTH + x0_offset;
            for obstacle_idx in 0..n_obstacle {
                let x = x0 + obstacle_idx * x_step;
                if let Some(buf) = self.buffer.get_mut(x..x+width) {
                    buf.fill(Tile::Obstacle);
                }
            }
        }
    }

    fn tick(&mut self, key_event: KeyEvent) {
        const MOVE_SIZE: i32 = MULT as i32;

        let player_move_diff = if key_event.pressed_left() { -1 }
        else if key_event.pressed_right() { 1 }
        else {0};

        self.player.try_move(player_move_diff * MOVE_SIZE);
        if key_event.pressed_space() {
            self.bullets.push_back(Bullet { x: self.player.pos as u8, y: (HEIGHT as u32 - PLAYER_BITMAP.height - 1) as u8, speed: -1, status: BulletStatus::Alive });
        }
        let shooting_enemy_idx = self.get_random_u32() % (MAX_ENEMIES as u32 * 4);
        if let Some(enemy) = self.enemies.get(shooting_enemy_idx as usize) {
            self.bullets.push_back(Bullet { x: enemy.x, y: enemy.y + (1 + ENEMY_BITMAP.height/2) as u8, speed: 1, status: BulletStatus::Alive });
        }

        let mut enemy_idx = 0;
        let mut head_x = WIDTH as u8 / 2;
        while let Some(enemy) = self.enemies.get_mut(enemy_idx) {
            if self.moving_right {
                enemy.x += ENEMY_SPEED;
                if enemy.x > head_x {
                    head_x = enemy.x;
                }
            } else {
                enemy.x -= ENEMY_SPEED;
                if enemy.x < head_x {
                    head_x = enemy.x;
                }
            }
            enemy_idx += 1;
        }
        if (self.moving_right && head_x > (WIDTH as u8 - 20)) ||
           (!self.moving_right && head_x < (20)) {
            self.moving_right = !self.moving_right;

            let mut head_y = 0;
            enemy_idx = 0;
            while let Some(enemy) = self.enemies.get_mut(enemy_idx) {
                if enemy.y > head_y {
                    head_y = enemy.y;
                }
                enemy.y += 4 * ENEMY_SPEED;
                enemy_idx += 1;
            }

            if head_y > HEIGHT as u8 - 40 {
                // YOU LOSE!
                self.game_state = GameState::EndScreen(false);
            }
        }

        if self.enemies.size() == 0 {
            // YOU WIN!
            self.game_state = GameState::EndScreen(true);
        }
    }

    fn update_buffer(&mut self) {
        for it in self.buffer.iter_mut() {
            if *it != Tile::Obstacle {
                *it = Tile::Background
            }
        }
        self.player.update(self.buffer);
        for enemy in self.enemies.iter() {
            enemy.update(self.buffer);
        }
        let mut idx = 0;
        while let Some(bullet) = self.bullets.get_mut(idx) {
            bullet.update(self.buffer);
            idx += 1;
        }
        let mut idx = (self.bullets.size() - 1) as isize;
        while idx >= 0 {
            if let Some(bullet) = self.bullets.get(idx as usize) {
                match bullet.status {
                    BulletStatus::HitPlayer => {
                        self.player.health -= 1;
                        self.player.color ^= 0x00_FF_00_00;
                    },
                    BulletStatus::HitEnemy => {
                        for (enemy_idx, enemy) in self.enemies.iter().enumerate() {
                            let x_dist = enemy.x as i32 - bullet.x as i32;
                            let (x_dist, _) = x_dist.overflowing_abs();
                            let y_dist = enemy.y as i32 - bullet.y as i32;
                            let (y_dist, _) = y_dist.overflowing_abs();
                            if x_dist <= ENEMY_BITMAP.width as i32/2 && y_dist <= ENEMY_BITMAP.height as i32/2 {
                                if let Some(enemy) = self.enemies.get_mut(enemy_idx) {
                                    enemy.health -= 1;
                                    break;
                                }
                            }
                        }
                    },
                    BulletStatus::Alive | BulletStatus::HitObstacle => (),
                }
                match bullet.status {
                    BulletStatus::Alive => (),
                    _ => self.bullets.remove(idx as usize),
                }
            }
            idx -= 1;
        }

        let mut idx = (self.enemies.size() - 1) as isize;
        while idx >= 0 {
            if let Some(enemy) = self.enemies.get(idx as usize) {
                if enemy.health <=0 {
                    self.enemies.remove(idx as usize);
                }
            }
            idx -= 1;
        }

        if self.player.health <= 0 {
            // YOU LOSE!
            self.game_state = GameState::EndScreen(false);
        }

    }

    fn render(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        for (idx, tile) in self.buffer.iter().enumerate() {
            let color = match tile {
                Tile::Background => self.default_color,
                Tile::Player => self.player.color,
                Tile::Bullet => 0xFF_80_80_80,
                Tile::Obstacle => 0xFF_E0_E0_E0,
                Tile::Enemy(val) => 0xFF_00_00_00 | ((*val as u32) << 22) | ((*val as u32) << 14) | ((*val as u32) << 6),
            };
            let buffer_row = idx / WIDTH;
            let buffer_col = idx % WIDTH;
            for row in 0..MULT {
                let ind0 = ((buffer_row * MULT) + row) * MULT * WIDTH + buffer_col * MULT;
                for col in 0..MULT {
                    if let Some(x) = js_buffer.get_mut(ind0 + col) {
                        *x = color;
                    }
                }
            }
        }
    }

    fn draw_start_screen(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        const START_SCREEN_COLOR: u32 = 0xFF_88_00_00;
        js_buffer.fill(START_SCREEN_COLOR);
    }

    fn draw_end_screen(&self, has_won: bool, js_buffer: &mut [u32; BUFFER_SIZE]) {
        const END_SCREEN_WINNER_COLOR: u32 = 0xFF_00_88_00;
        const END_SCREEN_LOSER_COLOR: u32 = 0xFF_00_00_88;
        let color = if has_won { END_SCREEN_WINNER_COLOR } else { END_SCREEN_LOSER_COLOR };
        js_buffer.fill(color);
    }

    fn get_random_u32(&mut self) -> u32 {
        self.random_seed = self.random_seed.wrapping_mul(1664525);
        self.random_seed = self.random_seed.wrapping_add(1013904223);
        self.random_seed
    }
}

impl Player {
    fn try_move(&mut self, diff: i32) {
        let new_pos = self.pos + diff;
        let width = PLAYER_BITMAP.width as i32;
        if new_pos >= (width / 2) && new_pos <= (WIDTH as i32 - width / 2) {
            self.pos = new_pos;
        }
    }

    fn update(&self, buffer: &mut [Tile; WIDTH * HEIGHT]) {
        let x0 = self.pos as u32 - PLAYER_BITMAP.width / 2;
        let mut y = HEIGHT as u32 - PLAYER_BITMAP.height;
        for row in PLAYER_BITMAP.bitmap.iter() {
            // Player's bitmap is contiguous so I can do this
            let offset = row.leading_zeros();
            let length = row.count_ones();
            
            let start_pos = (y * (WIDTH as u32) + x0 + offset) as usize;
            if let Some(x) = buffer.get_mut(start_pos..start_pos+length as usize) {
                x.fill(Tile::Player);
            }
            y += 1;
        }
    }
}

impl Enemy {
    fn update(&self, buffer: &mut [Tile; WIDTH * HEIGHT]) {
        let enemy_tile = Tile::Enemy(self.health);
        let x0 = self.x as u32 - ENEMY_BITMAP.width / 2;
        let y0 = self.y as u32 - ENEMY_BITMAP.height / 2;
        for (row_idx, &row) in ENEMY_BITMAP.bitmap.iter().enumerate() {
            let row_start = ((y0 + row_idx as u32) * (WIDTH as u32) + x0) as usize;
            for bit_idx in 0..15 {
                if row & (1 << (16 - bit_idx)) != 0 {
                    if let Some(x) = buffer.get_mut(row_start + bit_idx) {
                        *x = enemy_tile;
                    }
                }
            }
        }
    }
}

impl Bullet {
    fn update(&mut self, buffer: &mut [Tile; WIDTH * HEIGHT]) {
        if self.y == 0 || self.y == HEIGHT as u8 {
            self.status = BulletStatus::HitObstacle;
            return;
        }
        self.y = (self.y as i16 + self.speed as i16) as u8;
        let pos = self.y as usize * WIDTH + self.x as usize;
        if let Some(x) = buffer.get_mut(pos) {
            match *x {
                Tile::Background | Tile::Bullet => *x = Tile::Bullet,
                Tile::Player => self.status = BulletStatus::HitPlayer,
                Tile::Enemy(_) => {
                    if self.speed > 0 {
                        // Enemy hit enemy
                        self.status = BulletStatus::HitObstacle;
                    } else {
                        self.status = BulletStatus::HitEnemy;
                    }
                },
                Tile::Obstacle => {
                    *x = Tile::Background;
                    self.status = BulletStatus::HitObstacle;
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 420;
        assert_eq!(result, 420);
    }
}
