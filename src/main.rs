use ::std::thread;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::video::Window;
use std::cell::RefCell;
use std::fmt::Debug;
use std::time::Duration;

const FPS: u32 = 10;
const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;
const WINDOW_TITLE: &'static str = "TEST";
const BOARD_WIDTH: usize = 190;
const BOARD_HEIGHT: usize = 40;

pub struct Ctx<T: CellState> {
    pub canvas: Canvas<Window>,
    pub game: Game<T>,
}

#[allow(unused)]
#[derive(Debug)]
pub struct Game<T: CellState> {
    pub cells: Vec<Vec<Cell<T>>>,
    next_cells: Vec<Vec<Cell<T>>>,
    width: usize,
    height: usize,
    rect_width: u32,
    rect_height: u32,
    gap_x: i32,
    gap_y: i32,
    offset_x: i32,
    offset_y: i32,
    surround: RefCell<Vec<usize>>,
    mode: Option<T>,
}

#[derive(Debug)]
pub struct Cell<T: CellState> {
    pub state: T,
    pub rect: Rect,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum Seeds {
    #[default]
    White,
    Gray,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum GOL {
    #[default]
    White,
    Gray,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum BriansBrain {
    #[default]
    White,
    Red,
    Gray,
}

impl CellState for BriansBrain {
    fn color(&self) -> Color {
        match self {
            Self::White => Color::WHITE,
            Self::Gray => Color::GRAY,
            Self::Red => Color::RED,
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::White => Self::Gray,
            Self::Gray => Self::Red,
            Self::Red => Self::White,
        }
    }

    fn num() -> usize {
        3
    }

    fn transition(self, surround: &[usize]) -> Self {
        match surround {
            [_, _, 2, 0] => Self::Gray,
            [_, _, _, 2] => Self::Red,
            [_, _, _, 1] => Self::White,
            _ => Self::White,
        }
    }
}

impl From<usize> for BriansBrain {
    fn from(n: usize) -> Self {
        match n {
            0 => Self::White,
            1 => Self::Red,
            2 => Self::Gray,
            _ => panic!("Out of bounds {n}"),
        }
    }
}

impl CellState for Seeds {
    fn color(&self) -> Color {
        match self {
            Self::White => Color::WHITE,
            Self::Gray => Color::GRAY,
        }
    }

    fn toggle(self) -> Self {
        match self {
            Self::White => Self::Gray,
            Self::Gray => Self::White,
        }
    }

    fn num() -> usize {
        2
    }

    fn transition(self, surround: &[usize]) -> Self {
        match surround {
            [_, 2, _] => Self::Gray,
            _ => Self::White,
        }
    }
}

impl From<usize> for Seeds {
    fn from(n: usize) -> Self {
        match n {
            0 => Seeds::White,
            1 => Seeds::Gray,
            _ => panic!("Out of bounds {n}"),
        }
    }
}

impl CellState for GOL {
    fn color(&self) -> Color {
        match self {
            GOL::White => Color::WHITE,
            GOL::Gray => Color::GRAY,
        }
    }

    fn toggle(self) -> Self {
        match self {
            GOL::White => GOL::Gray,
            GOL::Gray => GOL::White,
        }
    }

    fn num() -> usize {
        2
    }

    fn transition(self, surround: &[usize]) -> Self {
        // GOL
        // Dead   Alive  Cur
        // [6,    2,     1]
        // [6,    3,     0]
        // [5,    3,     1]
        //
        // Not allowed
        // [4,    5,     0]
        match surround {
            [6, 2, 1] | [_, 3, _] => Self::Gray,
            _ => Self::White,
        }
    }
}

impl From<usize> for GOL {
    fn from(n: usize) -> Self {
        match n {
            0 => GOL::White,
            1 => GOL::Gray,
            _ => panic!("Out of bounds {n}"),
        }
    }
}

pub trait CellState: Debug + Clone + Copy + Eq + PartialEq + Default + From<usize> {
    fn color(&self) -> Color;
    fn toggle(self) -> Self;
    fn num() -> usize;
    fn transition(self, surround: &[usize]) -> Self;
    fn place_mode(self) -> Self {
        self.toggle()
    }
}

impl<T: CellState> Game<T> {
    pub fn new(width: usize, height: usize) -> Game<T> {
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
                        state: T::default(),
                        rect: Rect::new(x(i), y(j), rect_width, rect_height),
                    })
                    .collect()
            })
            .collect();

        let next_cells = (0..width)
            .map(|i| {
                (0..height)
                    .map(|j| Cell {
                        state: T::default(),
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
            surround: RefCell::new(vec![0; T::num() + 1]),
            mode: None,
        }
    }

    pub fn clear(&mut self) {
        {
            for (idx_x, i) in self.next_cells.iter().enumerate() {
                for (idx_y, _) in i.iter().enumerate() {
                    self.cells[idx_x][idx_y].state = T::default();
                }
            }
        }
        self.next_state();
    }

    pub fn toggle_state(&mut self, x: i32, y: i32) -> bool {
        let idx_x = (x - self.offset_x) / self.gap_x;
        let idx_y = (y - self.offset_y) / self.gap_y;

        self.cells
            .get_mut(idx_x as usize)
            .and_then(|i| {
                i.get_mut(idx_y as usize).and_then(|cell| {
                    let mode = self.mode.get_or_insert_with(|| cell.state.place_mode());
                    cell.rect.contains_point((x, y)).then(|| cell.switch(*mode))
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
            for (idx_y, cell) in i.iter().enumerate() {
                // GOL
                // Dead   Alive  Cur
                // [6,    2,     1]
                // [6,    3,     0]
                // [5,    3,     1]
                //
                // Not allowed
                // [4,    5,     0]
                self.update_surround(idx_x as isize, idx_y as isize);
                self.next_cells[idx_x][idx_y].state =
                    cell.state.transition(&self.surround.borrow());
            }
        }

        self.cells.swap_with_slice(&mut self.next_cells);
    }

    fn cell_is_state(&self, idx_x: isize, idx_y: isize, state: T) -> bool {
        self.cell(idx_x, idx_y)
            .and_then(|cell| cell.option_state(state))
            .is_some()
    }

    fn update_surround(&self, idx_x: isize, idx_y: isize) -> () {
        let surround = &mut self.surround.borrow_mut();
        for v in surround.iter_mut() {
            *v = 0;
        }

        for i in 0..T::num() {
            surround[i] += self.cell_is_state(idx_x - 1, idx_y - 1, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x - 0, idx_y - 1, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x + 1, idx_y - 1, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x - 1, idx_y - 0, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x + 1, idx_y - 0, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x - 1, idx_y + 1, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x - 0, idx_y + 1, T::from(i)) as usize;
            surround[i] += self.cell_is_state(idx_x + 1, idx_y + 1, T::from(i)) as usize;
        }

        surround[T::num()] += (1..T::num()).fold(0, |acc, n| {
            acc + self.cell_is_state(idx_x + 0, idx_y + 0, T::from(n)) as usize * n
        });
    }

    fn cell(&self, idx_x: isize, idx_y: isize) -> Option<&Cell<T>> {
        let (idx_x, idx_y) = self.wrap_idx(idx_x, idx_y);
        self.cells
            .get(idx_x)
            .and_then(|i: &Vec<Cell<T>>| i.get(idx_y).and_then(|cell| Some(cell)))
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

impl<T: CellState> Cell<T> {
    pub fn option_state(&self, state: T) -> Option<()> {
        (self.state == state).then_some(())
    }

    pub fn color(&self) -> Color {
        self.state.color()
    }

    pub fn switch(&mut self, state: T) {
        self.state = state;
    }

    pub fn toggle(&mut self) {
        self.state = self.state.toggle()
    }

    pub fn is_state(&self, state: T) -> bool {
        self.state.eq(&state)
    }
}

pub fn draw_grid<T: CellState>(ctx: &mut Ctx<T>) {
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
        game: Game::<GOL>::new(BOARD_WIDTH, BOARD_HEIGHT),
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
                Event::KeyDown {
                    keycode: Some(Keycode::C),
                    ..
                } => {
                    ctx.game.clear();
                }
                _ => {}
            }
        }
        draw_grid(&mut ctx);
        ctx.canvas.present();
        thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS));
    }
}
