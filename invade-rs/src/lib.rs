const WIDTH: usize = 600;
const HEIGHT: usize = 600;

#[no_mangle]
static mut BUFFER: [u32; WIDTH * HEIGHT] = [0; WIDTH * HEIGHT];

pub struct Game {
    default_color: u32,
    player: Player,
}

struct Player {
    pos: u32,
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

static mut GAME: Game = Game{ default_color: 0xFF_40_40_40, player: Player { pos: (WIDTH as u32)/2, width: 40, height: 20, color: 0xFF_00_00_FF }};

#[no_mangle]
pub unsafe extern fn js_game_tick(key_event_flags: u32) {
    GAME.tick(key_event_flags);
    GAME.render(&mut BUFFER);
}

impl Game {
    fn tick(&mut self, key_event_flags: u32) {
        if (key_event_flags & 1) != 0 {
            self.player.pos -= 10;
        }
        if (key_event_flags & 2) != 0 {
            self.player.pos += 10;
        }
        if (key_event_flags & 4) != 0 {
            self.player.color ^= 0x00_FF_00_00;
        }
    }

    fn render(&self, buffer: &mut [u32; WIDTH * HEIGHT]) {
        buffer.fill(self.default_color);
        self.render_player(buffer)
    }
    fn render_player(&self, buffer: &mut [u32; WIDTH * HEIGHT]) {
        self.player.render(buffer);
    }
}

impl Player {
    fn render(&self, buffer: &mut [u32; WIDTH * HEIGHT]) {
        static OFFSETS: [u32; 20] = [6*3, 6*3, 5*3, 5*3, 5*3, 5*3, 5*3, 5*3, 1*3, 1*3, 1*3, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        static WIDTHS: [u32; 20] = [1*4, 1*4, 1*4+2*3, 1*4+2*3, 1*4+2*3, 1*4+2*3, 1*4+2*3, 1*4+2*3, 1*4+10*3, 1*4+10*3, 1*4+10*3, 40, 40, 40, 40, 40, 40, 40, 40, 40];
        let x0 = self.pos - self.width/2;
        let y_end =  HEIGHT as u32;
        let y_start =  y_end - self.height;
        for y in y_start..y_end {
            let arr_idx = y - y_start;
            let offset = OFFSETS.get(arr_idx as usize).unwrap_or(&0);
            let width = WIDTHS.get(arr_idx as usize).unwrap_or(&0);

            let start_pos = (y * (WIDTH as u32) + x0 + offset) as usize;
            if let Some(x) = buffer.get_mut(start_pos..start_pos+*width as usize) {
                x.fill(self.color);
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
