mod static_allocator;

use std::cell::OnceCell;

const WIDTH: usize = 200;
const HEIGHT: usize = 200;
const MULT: usize = 3;
const BUFFER_SIZE: usize = WIDTH * MULT * HEIGHT * MULT;

#[no_mangle]
static mut BUFFER: [u32; BUFFER_SIZE] = [0; BUFFER_SIZE];


/*
 *       OO     
 *      OOOO    
 *      OOOO    
 *  OOOOOOOOOOOO 
 * OOOOOOOOOOOOOO
 * OOOOOOOOOOOOOO
 * OOOOOOOOOOOOOO
 * */
struct Player {
    pos: i32,
    width: u32,
    height: u32,
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
    game.player = Player { pos: (WIDTH as i32)/2, width: 14, height: 7, color: 0xFF_00_00_FF };
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
        if new_pos >= (self.width as i32 / 2) && new_pos <= (WIDTH as i32 - (self.width as i32) / 2) {
            self.pos = new_pos;
        }
    }

    fn update(&self, buffer: &mut [Tile; WIDTH * HEIGHT]) {
        static OFFSETS: [u32; 7] = [6, 5, 5, 1, 0, 0, 0];
        static WIDTHS: [u32; 7] = [2, 4, 4, 12, 14, 14, 14];
        let x0 = self.pos as u32 - self.width/2;
        let y_end = HEIGHT as u32;
        let y_start = y_end - self.height;
        for y in y_start..y_end {
            let arr_idx = y - y_start;
            let offset = OFFSETS.get(arr_idx as usize).unwrap_or(&0);
            let width = WIDTHS.get(arr_idx as usize).unwrap_or(&0);

            let start_pos = (y * (WIDTH as u32) + x0 + offset) as usize;
            if let Some(x) = buffer.get_mut(start_pos..start_pos+*width as usize) {
                x.fill(Tile::Player);
            }
        }
    }
}

impl Enemy {
    fn update(&self, buffer: &mut [Tile; WIDTH * HEIGHT]) {
        static ENEMY_WIDTH: u32 = 12;
        static ENEMY_HEIGHT: u32 = 8;
        let x0 = self.x as u32 - ENEMY_WIDTH/2;
        let y_start = self.y as u32 - ENEMY_HEIGHT/2;

        let enemy_tile = Tile::Enemy(self.health);

        let row_start = (y_start * (WIDTH as u32) + x0) as usize;
        if let Some(x) = buffer.get_mut(row_start + 2 as usize) {
            *x = enemy_tile;
        }
        if let Some(x) = buffer.get_mut(row_start + 9 as usize) {
            *x = enemy_tile;
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start + 3 as usize) {
            *x = enemy_tile;
        }
        if let Some(x) = buffer.get_mut(row_start + 8 as usize) {
            *x = enemy_tile;
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start+2..row_start+10 as usize) {
            x.fill(enemy_tile);
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start+1..row_start+3 as usize) {
            x.fill(enemy_tile);
        }
        if let Some(x) = buffer.get_mut(row_start+4..row_start+8 as usize) {
            x.fill(enemy_tile);
        }
        if let Some(x) = buffer.get_mut(row_start+9..row_start+11 as usize) {
            x.fill(enemy_tile);
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start..row_start+12 as usize) {
            x.fill(enemy_tile);
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start as usize) {
            *x = enemy_tile;
        }
        if let Some(x) = buffer.get_mut(row_start+2..row_start+10 as usize) {
            x.fill(enemy_tile);
        }
        if let Some(x) = buffer.get_mut(row_start+11 as usize) {
            *x = enemy_tile;
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start as usize) {
            *x = enemy_tile;
        }
        if let Some(x) = buffer.get_mut(row_start+2 as usize) {
            *x = enemy_tile;
        }
        if let Some(x) = buffer.get_mut(row_start+9 as usize) {
            *x = enemy_tile;
        }
        if let Some(x) = buffer.get_mut(row_start+11 as usize) {
            *x = enemy_tile;
        }

        let row_start = row_start + WIDTH;
        if let Some(x) = buffer.get_mut(row_start+3..row_start+5 as usize) {
            x.fill(enemy_tile);
        }
        if let Some(x) = buffer.get_mut(row_start+7..row_start+9 as usize) {
            x.fill(enemy_tile);
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
