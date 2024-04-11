mod static_allocator;

const WIDTH: usize = 200;
const HEIGHT: usize = 200;
const MULT: usize = 3;
const BUFFER_SIZE: usize = WIDTH * MULT * HEIGHT * MULT;

#[no_mangle]
static mut BUFFER: [u32; BUFFER_SIZE] = [0; BUFFER_SIZE];


pub struct Game<'a> {
    default_color: u32,
    player: Player,
    buffer: &'a mut [Tile; WIDTH * HEIGHT],
}

struct Player {
    pos: i32,
    width: u32,
    height: u32,
    color: u32,
}
/*
 *       O      
 *      OOO     
 *      OOO     
 *  OOOOOOOOOOO 
 * OOOOOOOOOOOOO
 * OOOOOOOOOOOOO
 * OOOOOOOOOOOOO
 * */

#[derive(Clone)]
enum Tile {
    Background,
    Player,
    Enemy,
}
const DEFAULT_TILE: Tile = Tile::Background;

static mut GAME: Game = Game{ default_color: 0xFF_FF_FF_FF, player: Player { pos: (WIDTH as i32)/2, width: 14, height: 7, color: 0xFF_00_00_FF }, buffer: &mut [DEFAULT_TILE; WIDTH * HEIGHT]};

//#[no_mangle]
//pub unsafe extern fn js_game_init() {
//}

#[no_mangle]
pub unsafe extern fn js_game_tick(key_event_flags: u32) {
    GAME.tick(key_event_flags);
    GAME.update_buffer();
    GAME.render(&mut BUFFER);
}

impl Game<'_> {
    fn tick(&mut self, key_event_flags: u32) {
        const MOVE_SIZE: i32 = MULT as i32;

        let player_move_diff = if (key_event_flags & 1) != 0 { -1 }
        else if (key_event_flags & 2) != 0 { 1 }
        else {0};

        self.player.try_move(player_move_diff * MOVE_SIZE);
        if (key_event_flags & 4) != 0 {
            self.player.color ^= 0x00_FF_00_00;
        }
    }

    fn update_buffer(&mut self) {
        self.buffer.fill(Tile::Background);
        self.player.update(self.buffer);
    }

    fn render(&self, js_buffer: &mut [u32; BUFFER_SIZE]) {
        for (idx, tile) in self.buffer.iter().enumerate() {
            let color = match tile {
                Tile::Background => self.default_color,
                Tile::Player => self.player.color,
                Tile::Enemy => self.default_color,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 420;
        assert_eq!(result, 420);
    }
}
