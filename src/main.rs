use ::std::thread;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::ops::Index;
use std::time::Duration;

const FPS: u32 = 60;
const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;
const WINDOW_TITLE: &'static str = "TEST";

pub struct Ctx {
    pub canvas: Canvas<Window>,
    pub game: Game,
}

#[allow(unused)]
#[derive(Debug)]
pub struct Game {
    pub cells: Vec<Vec<Cell>>,
    next_cells: Vec<Vec<Cell>>,
    width: usize,
    height: usize,
    rect_width: u32,
    rect_height: u32,
    gap_x: i32,
    gap_y: i32,
    offset_x: i32,
    offset_y: i32,
    mode: Option<Mode>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Mode {
    Place,
    Destroy,
}

#[derive(Debug)]
pub struct Cell {
    pub state: CellState,
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CellState {
    White = 0,
    Gray = 1,
}

impl Index<usize> for CellState {
    type Output = CellState;

    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &CellState::White,
            1 => &CellState::Gray,
            _ => panic!("Out of bounds {index}"),
        }
    }
}

impl Game {
    pub fn new(width: usize, height: usize) -> Game {
        const GAPSET: i32 = 0;
        const PAD: u32 = 100;

        let grid_max = width.max(height);
        let window_min = WINDOW_WIDTH.min(WINDOW_HEIGHT) - (GAPSET * grid_max as i32) as u32 - PAD;

        let rect_width = window_min / grid_max as u32;
        let rect_height = rect_width;

        let gap_x = rect_width as i32 + GAPSET;
        let gap_y = rect_height as i32 + GAPSET;

        let offset_x = (GAPSET + WINDOW_WIDTH as i32 - gap_x * width as i32) / 2;
        let offset_y = (GAPSET + WINDOW_HEIGHT as i32 - gap_y * height as i32) / 2;

        let x = |n| offset_x + n as i32 * gap_x;
        let y = |n| offset_y + n as i32 * gap_y;

        let cells = (0..width)
            .map(|i| {
                (0..height)
                    .map(|j| Cell {
                        state: CellState::White,
                        rect: Rect::new(x(i), y(j), rect_width, rect_height),
                    })
                    .collect()
            })
            .collect();

        let next_cells = (0..width)
            .map(|i| {
                (0..height)
                    .map(|j| Cell {
                        state: CellState::White,
                        rect: Rect::new(x(i), y(j), rect_width, rect_height),
                    })
                    .collect()
            })
            .collect();

        Game {
            cells,
            next_cells,
            width,
            height,
            rect_width,
            rect_height,
            gap_x,
            gap_y,
            offset_x,
            offset_y,
            mode: None,
        }
    }

    pub fn toggle_state(&mut self, x: i32, y: i32) -> bool {
        let idx_x = (x - self.offset_x) / self.gap_x;
        let idx_y = (y - self.offset_y) / self.gap_y;

        self.cells
            .get_mut(idx_x as usize)
            .and_then(|i| {
                i.get_mut(idx_y as usize).and_then(|cell| {
                    let mode = self.mode.get_or_insert_with(|| match cell.state {
                        CellState::White => Mode::Place,
                        CellState::Gray => Mode::Destroy,
                    });
                    cell.rect.contains_point((x, y)).then(|| match mode {
                        Mode::Place => cell.switch(CellState::Gray),
                        Mode::Destroy => cell.switch(CellState::White),
                    })
                })
            })
            .is_some()

        //Naive solution
        //for (idx_x, i) in cells.iter_mut().enumerate() {
        //    for (idx_y, cell) in i.iter_mut().enumerate() {
        //        if cell.rect.contains_point((x, y)) {
        //            dbg!(idx_x);
        //            dbg!(idx_y);
        //            cell.toggle();
        //            return true;
        //        }
        //    }
        //}
        //false
    }

    pub fn next_state(&mut self) {
        for (idx_x, i) in self.cells.iter().enumerate() {
            for (idx_y, _) in i.iter().enumerate() {
                // Check
                // * * *
                // * o *
                // * * *
                // x - 1, y - 1
                // x - 0, y - 1
                // x + 1, y - 1
                // ...

                // GOL
                // Dead   Alive  Cur
                // [6,    2,     1]
                // [6,    3,     0]
                // [5,    3,     1]
                //
                // Not allowed
                // [4,    5,     0]

                match self.get_surround(CellState::Gray, idx_x as isize, idx_y as isize) {
                    [6, 2, 1] | [_, 3, _] => self.next_cells[idx_x][idx_y].state = CellState::Gray,
                    _ => self.next_cells[idx_x][idx_y].state = CellState::White,
                }
            }
        }

        self.cells.swap_with_slice(&mut self.next_cells);
    }

    fn cell_is_state(&self, idx_x: isize, idx_y: isize, state: CellState) -> bool {
        self.cell(idx_x, idx_y)
            .and_then(|cell| cell.option_state(state))
            .is_some()
    }

    fn get_surround(&self, state: CellState, idx_x: isize, idx_y: isize) -> [usize; 3] {
        let mut surround = [0, 0, 0];

        for i in 0..2 {
            surround[i] += self.cell_is_state(idx_x - 1, idx_y - 1, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x - 0, idx_y - 1, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x + 1, idx_y - 1, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x - 1, idx_y - 0, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x + 1, idx_y - 0, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x - 1, idx_y + 1, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x - 0, idx_y + 1, state[i]) as usize;
            surround[i] += self.cell_is_state(idx_x + 1, idx_y + 1, state[i]) as usize;
        }

        surround[2] += self
            .cell(idx_x + 0, idx_y + 0)
            .and_then(|cell| cell.option_state(state[1]))
            .is_some() as usize;
        surround
    }

    fn cell(&self, idx_x: isize, idx_y: isize) -> Option<&Cell> {
        let (idx_x, idx_y) = self.wrap_idx(idx_x, idx_y);
        self.cells
            .get(idx_x)
            .and_then(|i: &Vec<Cell>| i.get(idx_y).and_then(|cell| Some(cell)))
    }

    fn wrap_idx(&self, idx_x: isize, idx_y: isize) -> (usize, usize) {
        let x = (idx_x % (self.width as isize) < 0)
            .then(|| self.width as isize - 1)
            .unwrap_or(idx_x % self.width as isize);
        let y = (idx_y % (self.height as isize) < 0)
            .then(|| self.height as isize - 1)
            .unwrap_or(idx_y % self.height as isize);

        (x as usize, y as usize)
    }

    pub fn width(&self) -> usize {
        self.cells.len()
    }

    pub fn height(&self) -> usize {
        self.cells[0].len()
    }
}

impl Cell {
    pub fn option_state(&self, state: CellState) -> Option<()> {
        (self.state == state).then_some(())
    }

    pub fn color(&self) -> Color {
        self.state.color()
    }

    pub fn switch(&mut self, state: CellState) {
        self.state = state;
    }

    pub fn toggle(&mut self) {
        self.state = self.state.toggle()
    }

    pub fn is_state(&self, state: CellState) -> bool {
        self.state.eq(&state)
    }
}

impl CellState {
    pub fn color(&self) -> Color {
        match self {
            CellState::White => Color::WHITE,
            CellState::Gray => Color::GRAY,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            CellState::White => CellState::Gray,
            CellState::Gray => CellState::White,
        }
    }
}

pub fn draw_grid(ctx: &mut Ctx) {
    let cells = &ctx.game.cells;
    for i in cells {
        for cell in i {
            ctx.canvas.set_draw_color(cell.color());
            ctx.canvas.fill_rect(cell.rect.clone()).unwrap();
        }
    }
}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window(WINDOW_TITLE, WINDOW_WIDTH, WINDOW_HEIGHT)
        .position_centered()
        .build()
        .unwrap();

    let canvas = window.into_canvas().build().unwrap();
    let mut ctx = Ctx {
        canvas,
        game: Game::new(20, 20),
    };

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        ctx.canvas.set_draw_color(Color::RGB(0, 0, 0));
        ctx.canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::MouseButtonDown { x, y, .. } => {
                    ctx.game.toggle_state(x, y);
                }
                Event::MouseMotion {
                    x, y, mousestate, ..
                } if mousestate.left() => {
                    ctx.game.toggle_state(x, y);
                }
                Event::MouseButtonUp { .. } => {
                    //ctx.game.prev_hold.clear();
                    ctx.game.mode = None;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    ..
                } => {
                    ctx.game.next_state();
                }
                _ => {}
            }
        }
        draw_grid(&mut ctx);
        ctx.canvas.present();
        thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS));
    }
}
