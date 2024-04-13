mod static_allocator;

use std::cell::OnceCell;

const WIDTH: usize = 200;
const HEIGHT: usize = 200;
const MULT: usize = 3;
const BUFFER_SIZE: usize = WIDTH * MULT * HEIGHT * MULT;

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
}

#[derive(Clone, Copy)]
enum Tile {
    Background,
    Player,
    Enemy(u8),
}
const DEFAULT_TILE: Tile = Tile::Background;
const ENEMY_SPEED: u8 = 1;

struct Enemy {
    x: u8,
    y: u8,
    health: u8,
}

struct Bullet {
    x: u8,
    y: u8,
    speed: u8
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
    game.player = Player { pos: (WIDTH as i32)/2, color: 0xFF_00_00_FF };
    game.enemies = static_allocator::SVector::new(16);
    game.bullets = static_allocator::SVector::new(128);
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
            GameState::EndScreen(_) => {
                game.draw_end_screen(&mut BUFFER);
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
        self.enemies.reset();
        self.bullets.reset();
        self.moving_right = true;
        self.enemies.push_back(Enemy { x: 20, y: 20, health: 10 });
        self.enemies.push_back(Enemy { x: 60, y: 20, health: 10 });
        self.enemies.push_back(Enemy { x: 40, y: 40, health: 10 });
    }

    fn tick(&mut self, key_event: KeyEvent) {
        const MOVE_SIZE: i32 = MULT as i32;

        let player_move_diff = if key_event.pressed_left() { -1 }
        else if key_event.pressed_right() { 1 }
        else {0};

        self.player.try_move(player_move_diff * MOVE_SIZE);
        if key_event.pressed_space() {
            self.player.color ^= 0x00_FF_00_00;
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
                enemy.y += ENEMY_SPEED;
                enemy_idx += 1;
            }

            if head_y > HEIGHT as u8 - 40 {
                // YOU LOSE!
                self.game_state = GameState::EndScreen(false);
            }
        }
    }

    fn update_buffer(&mut self) {
        self.buffer.fill(Tile::Background);
        self.player.update(self.buffer);
        for enemy in self.enemies.iter() {
            enemy.update(self.buffer);
        }
    }

    fn render(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        for (idx, tile) in self.buffer.iter().enumerate() {
            let color = match tile {
                Tile::Background => self.default_color,
                Tile::Player => self.player.color,
                Tile::Enemy(val) => 0xFF_00_00_00 | ((*val as u32) << 24) | ((*val as u32) << 16) | ((*val as u32) << 8),
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
        const START_SCREEN_COLOR: u32 = 0xFF_00_88_00;
        js_buffer.fill(START_SCREEN_COLOR);
    }

    fn draw_end_screen(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        const END_SCREEN_COLOR: u32 = 0xFF_00_FF_00;
        js_buffer.fill(END_SCREEN_COLOR);
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
        let x0 = self.pos as u32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 420;
        assert_eq!(result, 420);
    }
}
