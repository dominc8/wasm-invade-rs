mod static_allocator;

use std::cell::OnceCell;

const STATUS_BAR_HEIGHT: usize = 10;
const WIDTH: usize = 200;
const HEIGHT: usize = 150;
const MULT: usize = 6;
const BUFFER_SIZE: usize = WIDTH * MULT * (HEIGHT + STATUS_BAR_HEIGHT) * MULT;
const MAX_BULLETS: usize = 64;
const MAX_ENEMIES: usize = 16;
const MAX_PLAYER_HEALTH: i32 = 3;
const MAX_RIFLE_AMMO: i32 = 20;
const FONT_SIZE: u32 = 5;

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

enum Weapon {
    Pistol,
    Rifle,
}

struct Player {
    pos: i32,
    color: u32,
    health: i32,
    last_shot_in_ticks: u32,
    opacity: u32,
    weapon: Weapon,
    rifle_ammo: i32,
    reset_status_bar: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Tile {
    Background,
    Player,
    Bullet,
    Obstacle,
    Enemy(u8),
}
const DEFAULT_TILE: Tile = Tile::Background;
const ENEMY_SPEED_FREQ_IN_TICKS: u64 = 1;

struct Enemy {
    x: u8,
    y: u8,
    health: i8,
    max_health: u8,
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
    damage: u8,
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
    fn pressed_ctrl(&self) -> bool {
        (self.0 & 8) != 0
    }
    fn pressed_escape(&self) -> bool {
        (self.0 & 16) != 0
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
    tick_counter: u64,
    moving_right: bool,
    paused: bool,
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
    game.player = Player {
        pos: (WIDTH as i32)/2, color: 0xFF_00_00_FF, health: MAX_PLAYER_HEALTH,
        last_shot_in_ticks: 0, opacity: 100, weapon: Weapon::Pistol,
        rifle_ammo: MAX_RIFLE_AMMO, reset_status_bar: false
    };
    game.enemies = static_allocator::SVector::new(MAX_ENEMIES);
    game.bullets = static_allocator::SVector::new(MAX_BULLETS);
    game.tick_counter = 0;
    game.moving_right = true;
    game.paused = false;
    game.buffer = &mut GAMEBUFFER;

    let _ = GAMECELL.set(game);
}

#[no_mangle]
pub unsafe extern fn js_game_tick(key_event_flags: u32) {
    if let Some(game) = GAMECELL.get_mut() {
        let key_event = get_key_event(key_event_flags);

        if key_event.pressed_escape() {
            game.paused = !game.paused;

            if game.paused {
                game.draw_help_screen(&mut BUFFER);
            }
        }
        if game.paused { return };

        match game.game_state {
            GameState::StartScreen => {
                game.draw_start_screen(&mut BUFFER);
                if key_event.pressed_space() {
                    game.reset_level();
                    BUFFER.fill(0xFF_FF_FF_FF);
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
        self.player.reset();
        self.enemies.reset();
        self.bullets.reset();
        self.moving_right = true;
        self.tick_counter = 0;

        let enemy_step = 20;
        let n_enemy_in_row = WIDTH / 2 / enemy_step;
        let n_enemy_rows = 3;
        for row_idx in 0..n_enemy_rows {
            let y = (enemy_step * (row_idx + 1)) as u8;
            for col_idx in 0..n_enemy_in_row {
                let x = (enemy_step / 2 + col_idx * enemy_step) as u8;
                self.enemies.push_back(Enemy { x, y, health: 2, max_health: 2 });
            }
        }

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

        self.tick_counter = self.tick_counter.wrapping_add(1);
        self.player.tick();
        self.player.try_move(player_move_diff * MOVE_SIZE);
        if key_event.pressed_ctrl() {
            self.player.change_weapon();
        }
        if key_event.pressed_space() {
            if let Some(bullet) = self.player.try_shoot() {
                self.bullets.push_back(bullet);
            }
        }
        let shooting_enemy_idx = self.get_random_u32() % (MAX_ENEMIES as u32 * 4);
        if let Some(enemy) = self.enemies.get(shooting_enemy_idx as usize) {
            self.bullets.push_back(Bullet { x: enemy.x, y: enemy.y + (1 + ENEMY_BITMAP.height/2) as u8, speed: 1, damage: 1, status: BulletStatus::Alive });
        }

        let enemy_mov_horz: u8 = if self.tick_counter % ENEMY_SPEED_FREQ_IN_TICKS == 0 { 1 } else { 0 };
        let mut enemy_idx = 0;
        let mut head_x = WIDTH as u8 / 2;
        while let Some(enemy) = self.enemies.get_mut(enemy_idx) {
            if self.moving_right {
                enemy.x += enemy_mov_horz;
                if enemy.x > head_x {
                    head_x = enemy.x;
                }
            } else {
                enemy.x -= enemy_mov_horz;
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
                enemy.y += 4;
                enemy_idx += 1;
            }

            if head_y >= HEIGHT as u8 - PLAYER_BITMAP.height as u8 {
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
                        self.player.health -= bullet.damage as i32;
                        self.player.opacity = 10;
                    },
                    BulletStatus::HitEnemy => {
                        for (enemy_idx, enemy) in self.enemies.iter().enumerate() {
                            let x_dist = enemy.x as i32 - bullet.x as i32;
                            let (x_dist, _) = x_dist.overflowing_abs();
                            let y_dist = enemy.y as i32 - bullet.y as i32;
                            let (y_dist, _) = y_dist.overflowing_abs();
                            if x_dist <= ENEMY_BITMAP.width as i32/2 && y_dist <= ENEMY_BITMAP.height as i32/2 {
                                if let Some(enemy) = self.enemies.get_mut(enemy_idx) {
                                    enemy.health -= bullet.damage as i8;
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

        if self.player.opacity < 100 {
            self.player.opacity += 1;
        }

    }

    fn render(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        for (idx, tile) in self.buffer.iter().enumerate() {
            let color = match tile {
                Tile::Background => self.default_color,
                Tile::Player => {
                    let opacity_hex = (100 - self.player.opacity) * 255 / 100;
                    let mask = opacity_hex.wrapping_shl(16) | opacity_hex.wrapping_shl(8) | opacity_hex;
                    self.player.color | mask
                },
                Tile::Bullet => 0xFF_80_80_80,
                Tile::Obstacle => 0xFF_E0_E0_E0,
                Tile::Enemy(val) => {
                    let color = *val as u32;
                    0xFF_00_00_00 | color.wrapping_shl(16) | color.wrapping_shl(8) | color
                }
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
        self.render_status_bar(js_buffer);
    }

    fn render_health_bar(&self, js_buffer: &mut [u32; BUFFER_SIZE], offset: usize) -> usize {
        const TXT_COLOR: u32 = 0xFF_00_00_00;

        let health_string_start = offset;
        let health_string_end = self.render_text(js_buffer, "HP:", health_string_start, 1, TXT_COLOR);

        const HEALTH_COLORS: [u32; MAX_PLAYER_HEALTH as usize] = [
            0xFF_10_10_FF,
            0xFF_10_A0_FF,
            0xFF_10_FF_10,
        ];
        const BAR_WIDTH: usize = 3 * MULT;
        const BAR_STEP: usize = BAR_WIDTH + 1;
        let health_bar_start = health_string_end + BAR_STEP;
        let health_bar_end = health_bar_start + MAX_PLAYER_HEALTH as usize * BAR_STEP;
        let default_bar_color;
        if let Some(bar_color) = HEALTH_COLORS.get((self.player.health - 1) as usize) {
            default_bar_color = *bar_color;
        } else {
            default_bar_color = 0xFF_FF_FF_FF;
        }
        for row_idx in 0..(FONT_SIZE as usize * MULT) {
            let buffer_start = health_bar_start + row_idx * WIDTH * MULT;
            for bar_idx in 0..MAX_PLAYER_HEALTH as usize {
                let color;
                if (bar_idx as i32) < self.player.health {
                    color = default_bar_color;
                } else {
                    color = 0xFF_FF_FF_FF;
                }
                let ind0 = buffer_start + bar_idx * BAR_STEP;
                if let Some(x) = js_buffer.get_mut(ind0..ind0+BAR_WIDTH) {
                    x.fill(color);
                }

            }
        }
        return health_bar_end;
    }

    fn render_weapon_status(&self, js_buffer: &mut [u32; BUFFER_SIZE], offset: usize) -> usize {
        const TXT_COLOR: u32 = 0xFF_00_00_00;

        let weapon_string_start = offset;
        let weapon_string = match self.player.weapon {
            Weapon::Pistol => "PISTOL: ",
            Weapon::Rifle => "RIFLE: ",
        };
        let weapon_string_end = self.render_text(js_buffer, weapon_string, weapon_string_start, 1, TXT_COLOR);

        let weapon_ammo_start = weapon_string_end;
        let weapon_ammo_end = match self.player.weapon {
            Weapon::Pistol => self.render_inf_symbol(js_buffer, weapon_ammo_start, 1, TXT_COLOR),
            Weapon::Rifle => {
                // u32 to char array without panic
                let ammo_string = match self.player.rifle_ammo {
                    0 => "0/20",
                    1 => "1/20",
                    2 => "2/20",
                    3 => "3/20",
                    4 => "4/20",
                    5 => "5/20",
                    6 => "6/20",
                    7 => "7/20",
                    8 => "8/20",
                    9 => "9/20",
                    10 => "10/20",
                    11 => "11/20",
                    12 => "12/20",
                    13 => "13/20",
                    14 => "14/20",
                    15 => "15/20",
                    16 => "16/20",
                    17 => "17/20",
                    18 => "18/20",
                    19 => "19/20",
                    _ => "20/20",
                };
                self.render_text(js_buffer, &ammo_string, weapon_ammo_start, 1, TXT_COLOR)
            }
        };

        return weapon_ammo_end;
    }

    fn render_status_bar(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        let offset = (HEIGHT + 1) * MULT * WIDTH * MULT;
        if self.player.reset_status_bar {
            if let Some(x) = js_buffer.get_mut(offset..) {
                x.fill(0xFF_FF_FF_FF);
            }
        }
        let offset = self.render_health_bar(js_buffer, offset);
        let _offset = self.render_weapon_status(js_buffer, offset);
    }

    fn render_text_aligned(&self, js_buffer: &mut [u32; BUFFER_SIZE], text: &str, y: usize, scale: usize, color: u32) -> usize {
        let mut text_width = 0;
        for c in text.chars() {
            let bm = self.get_char_bitmap(c);
            text_width += (bm.width + 1) as usize * MULT * scale;
        }
        let x_offset = (WIDTH * MULT - text_width) / 2;
        let start_pos = y * WIDTH * MULT + x_offset;
        self.render_text(js_buffer, text, start_pos, scale, color)
    }

    fn render_text(&self, js_buffer: &mut [u32; BUFFER_SIZE], text: &str, start_pos: usize, scale: usize, color: u32) -> usize {
        let mut pos = start_pos;
        for c in text.chars() {
            pos = self.render_char(js_buffer, c, pos, scale, color);
        }
        return pos
    }

    fn render_char(&self, js_buffer: &mut [u32; BUFFER_SIZE], c: char, start_pos: usize, scale: usize, color: u32) -> usize {
        let bm = self.get_char_bitmap(c);

        for (row_idx, &row) in bm.bitmap.iter().enumerate() {
            let buffer_start = start_pos + row_idx * WIDTH * MULT * MULT * scale;
            for bit_idx in 0..16 {
                if row & (1 << (16 - bit_idx)) != 0 {
                    let ind0 = buffer_start + bit_idx*MULT*scale;
                    for i in 0..MULT*scale {
                        let ind0 = ind0 + i*WIDTH*MULT;
                        if let Some(x) = js_buffer.get_mut(ind0..ind0+MULT*scale) {
                            x.fill(color);
                        }
                    }
                }
            }
        }
        return start_pos + (bm.width + 1) as usize * MULT * scale
    }

    fn render_inf_symbol(&self, js_buffer: &mut [u32; BUFFER_SIZE], start_pos: usize, scale: usize, color: u32) -> usize {
        let bm = &Bitmap2D { width: 9, height: FONT_SIZE,
                    bitmap: &[
                        0b0110001100000000,
                        0b1001010010000000,
                        0b1000100010000000,
                        0b1001010010000000,
                        0b0110001100000000,
                    ] };

        for (row_idx, &row) in bm.bitmap.iter().enumerate() {
            let buffer_start = start_pos + row_idx * WIDTH * MULT * MULT * scale;
            for bit_idx in 0..16 {
                if row & (1 << (16 - bit_idx)) != 0 {
                    let ind0 = buffer_start + bit_idx*MULT*scale;
                    for i in 0..MULT*scale {
                        let ind0 = ind0 + i*WIDTH*MULT;
                        if let Some(x) = js_buffer.get_mut(ind0..ind0+MULT*scale) {
                            x.fill(color);
                        }
                    }
                }
            }
        }
        return start_pos + (bm.width + 1) as usize * MULT * scale
    }

    fn get_char_bitmap(&self, c: char) -> &Bitmap2D {
        match c {
            'A' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                        ] },
            'C' => &Bitmap2D { width: 3, height: FONT_SIZE,
                        bitmap: &[
                            0b1110000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1110000000000000,
                        ] },
            'D' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1110000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1110000000000000,
                        ] },
            'E' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1110000000000000,
                            0b1000000000000000,
                            0b1111000000000000,
                        ] },
            'F' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1110000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                        ] },
            'G' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1011000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                        ] },
            'H' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                        ] },
            'I' => &Bitmap2D { width: 1, height: FONT_SIZE,
                        bitmap: &[
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                        ] },
            'L' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                            0b1111000000000000,
                        ] },
            'M' => &Bitmap2D { width: 5, height: FONT_SIZE,
                        bitmap: &[
                            0b1000100000000000,
                            0b1101100000000000,
                            0b1010100000000000,
                            0b1000100000000000,
                            0b1000100000000000,
                        ] },
            'N' => &Bitmap2D { width: 5, height: FONT_SIZE,
                        bitmap: &[
                            0b1000100000000000,
                            0b1100100000000000,
                            0b1010100000000000,
                            0b1001100000000000,
                            0b1000100000000000,
                        ] },
            'O' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                        ] },
            'P' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1000000000000000,
                        ] },
            'R' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                            0b1010000000000000,
                            0b1001000000000000,
                        ] },
            'S' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1111000000000000,
                            0b0001000000000000,
                            0b1111000000000000,
                        ] },
            'T' => &Bitmap2D { width: 5, height: FONT_SIZE,
                        bitmap: &[
                            0b1111100000000000,
                            0b0010000000000000,
                            0b0010000000000000,
                            0b0010000000000000,
                            0b0010000000000000,
                        ] },
            'U' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                        ] },
            'W' => &Bitmap2D { width: 5, height: FONT_SIZE,
                        bitmap: &[
                            0b1000100000000000,
                            0b1000100000000000,
                            0b1010100000000000,
                            0b1101100000000000,
                            0b1000100000000000,
                        ] },
            'Y' => &Bitmap2D { width: 5, height: FONT_SIZE,
                        bitmap: &[
                            0b1000100000000000,
                            0b0101000000000000,
                            0b0010000000000000,
                            0b0010000000000000,
                            0b0010000000000000,
                        ] },
            '0' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b0110000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b1001000000000000,
                            0b0110000000000000,
                        ] },
            '1' => &Bitmap2D { width: 3, height: FONT_SIZE,
                        bitmap: &[
                            0b0100000000000000,
                            0b1100000000000000,
                            0b0100000000000000,
                            0b0100000000000000,
                            0b1110000000000000,
                        ] },
            '2' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b0110000000000000,
                            0b1001000000000000,
                            0b0010000000000000,
                            0b0100000000000000,
                            0b1111000000000000,
                        ] },
            '3' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b0110000000000000,
                            0b1001000000000000,
                            0b0010000000000000,
                            0b1001000000000000,
                            0b0110000000000000,
                        ] },
            '4' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b0101000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                            0b0001000000000000,
                            0b0001000000000000,
                        ] },
            '5' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1110000000000000,
                            0b0001000000000000,
                            0b1110000000000000,
                        ] },
            '6' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b1000000000000000,
                            0b1111000000000000,
                            0b1001000000000000,
                            0b1111000000000000,
                        ] },
            '7' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b1111000000000000,
                            0b0001000000000000,
                            0b0010000000000000,
                            0b0100000000000000,
                            0b0100000000000000,
                        ] },
            '8' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b0110000000000000,
                            0b1001000000000000,
                            0b0110000000000000,
                            0b1001000000000000,
                            0b0110000000000000,
                        ] },
            '9' => &Bitmap2D { width: 4, height: FONT_SIZE,
                        bitmap: &[
                            0b0110000000000000,
                            0b1001000000000000,
                            0b0111000000000000,
                            0b0010000000000000,
                            0b0100000000000000,
                        ] },
            '/' => &Bitmap2D { width: 5, height: FONT_SIZE,
                        bitmap: &[
                            0b0000100000000000,
                            0b0001000000000000,
                            0b0010000000000000,
                            0b0100000000000000,
                            0b1000000000000000,
                        ] },
            ':' => &Bitmap2D { width: 1, height: FONT_SIZE,
                        bitmap: &[
                            0b0000000000000000,
                            0b0000000000000000,
                            0b1000000000000000,
                            0b0000000000000000,
                            0b1000000000000000,
                        ] },
            ' ' => &Bitmap2D { width: 2, height: FONT_SIZE,
                        bitmap: &[
                        ] },
            _ => &Bitmap2D { width: 3, height: FONT_SIZE,
                        bitmap: &[
                            0b1110000000000000,
                            0b1110000000000000,
                            0b1110000000000000,
                            0b1110000000000000,
                            0b1110000000000000,
                        ] },
        }
    }

    fn draw_start_screen(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        const BG_COLOR: u32 = 0xFF_88_88_88;
        const TXT_COLOR: u32 = 0xFF_00_00_00;
        js_buffer.fill(BG_COLOR);
        self.render_text_aligned(js_buffer, "PRESS SPACE TO START", HEIGHT * MULT / 2, 2, TXT_COLOR);

        let offset = BUFFER_SIZE - WIDTH * MULT * MULT * (FONT_SIZE  + 1) as usize;
        self.render_text(js_buffer, "ESC: PAUSE/HELP MENU", offset, 1, TXT_COLOR);
    }

    fn draw_end_screen(&self, has_won: bool, js_buffer: &mut [u32; BUFFER_SIZE]) {
        const END_SCREEN_WINNER_COLOR: u32 = 0xFF_00_88_00;
        const END_SCREEN_LOSER_COLOR: u32 = 0xFF_00_00_88;
        const TXT_COLOR: u32 = 0xFF_00_00_00;
        let color = if has_won { END_SCREEN_WINNER_COLOR } else { END_SCREEN_LOSER_COLOR };
        let text = if has_won { "YOU WIN" } else { "YOU LOSE" };
        js_buffer.fill(color);
        self.render_text_aligned(js_buffer, text, HEIGHT * MULT / 2, 2, TXT_COLOR);
    }

    fn draw_help_screen(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        const BG_COLOR: u32 = 0xFF_AA_AA_AA;
        const TXT_COLOR: u32 = 0xFF_00_00_00;
        const TXT_SCALE: usize = 2;
        js_buffer.fill(BG_COLOR);
        self.render_text_aligned(js_buffer, "GAME PAUSED", HEIGHT * MULT / 6, TXT_SCALE, TXT_COLOR);
        let offset = (HEIGHT / 3) * MULT * WIDTH * MULT;// + (WIDTH / 4) * MULT;
        self.render_text(js_buffer, "SHOOT: SPACE", offset, TXT_SCALE, TXT_COLOR);
        let offset = offset + (FONT_SIZE as usize + 2) * TXT_SCALE * WIDTH * MULT * MULT;
        self.render_text(js_buffer, "TOGGLE WEAPON: CTRL", offset, TXT_SCALE, TXT_COLOR);
        let offset = offset + (FONT_SIZE as usize + 2) * TXT_SCALE * WIDTH * MULT * MULT;
        self.render_text(js_buffer, "PAUSE: ESC", offset, TXT_SCALE, TXT_COLOR);
    }

    fn get_random_u32(&mut self) -> u32 {
        self.random_seed = self.random_seed.wrapping_mul(1664525);
        self.random_seed = self.random_seed.wrapping_add(1013904223);
        self.random_seed
    }
}

impl Player {
    fn reset(&mut self) {
        self.pos = (WIDTH as i32)/2;
        self.health = 3;
        self.last_shot_in_ticks = 0;
        self.weapon = Weapon::Pistol;
        self.rifle_ammo = MAX_RIFLE_AMMO;
        self.reset_status_bar = false;
    }

    fn tick(&mut self) {
        self.last_shot_in_ticks += 1;
        self.reset_status_bar = false;
    }

    fn change_weapon(&mut self) {
        self.reset_status_bar = true;
        self.weapon = match self.weapon {
            Weapon::Pistol => Weapon::Rifle,
            Weapon::Rifle => Weapon::Pistol,
        };
    }

    fn try_move(&mut self, diff: i32) {
        let new_pos = self.pos + diff;
        let width = PLAYER_BITMAP.width as i32;
        if new_pos >= (width / 2) && new_pos <= (WIDTH as i32 - width / 2) {
            self.pos = new_pos;
        }
    }

    fn try_shoot(&mut self) -> Option<Bullet> {
        let (cooldown, damage, has_ammo) = match self.weapon {
            Weapon::Pistol => (15, 1, true),
            Weapon::Rifle => (30, 2, self.rifle_ammo > 0),
        };
        if !has_ammo || self.last_shot_in_ticks < cooldown {
            return None;
        }
        self.last_shot_in_ticks = 0;
        match self.weapon {
            Weapon::Pistol => (),
            Weapon::Rifle => {self.rifle_ammo -= 1; self.reset_status_bar = true;},
        };
        return Some(Bullet {
            x: self.pos as u8, y: (HEIGHT as u32 - PLAYER_BITMAP.height - 1) as u8,
            speed: -1, damage, status: BulletStatus::Alive
        })
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
        let max_health = if self.max_health == 0 { 1 } else { self.max_health as u32 };
        let color = 255 * (100 - (self.health as u32 * 100 / max_health));
        let enemy_tile = Tile::Enemy(color as u8);
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
