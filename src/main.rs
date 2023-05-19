#![allow(unused_imports)]
mod automatons;

use ::std::thread;
use automatons::{BB, Gol, Seeds};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, RenderTarget};
use sdl2::video::Window;
use std::fmt::Debug;
use std::thread::JoinHandle;
use std::time::Duration;

const FPS: u32 = 60;
const WINDOW_WIDTH: u32 = 1920;
const WINDOW_HEIGHT: u32 = 1080;
const WINDOW_TITLE: &str = "TEST";
const BOARD_WIDTH: usize = 100;
const BOARD_HEIGHT: usize = 100;

pub struct Ctx<T: CellState + Send + Sync> {
    pub canvas: Canvas<Window>,
    pub game: Game<T>,
    pub draw_threads: Vec<Option<JoinHandle<()>>>,
}

#[allow(unused)]
#[derive(Debug)]
pub struct Game<T: CellState + Send + Sync> {
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
    surround: Vec<Vec<u8>>,
    join_handles: Vec<Option<JoinHandle<()>>>,
    mode: Option<T>,
}

pub struct GamePtr<T: CellState + Send + Sync> {
    game: *const Game<T>,
}

unsafe impl<T: CellState + Send + Sync> Send for GamePtr<T> {}
unsafe impl<T: CellState + Send + Sync> Sync for GamePtr<T> {}

#[derive(Debug)]
pub struct Cell<T: CellState> {
    pub state: T,
    pub rect: Rect,
}

pub trait CellState: Debug + Clone + Copy + Eq + PartialEq + Default + From<usize> {
    fn color(&self) -> Color;
    fn toggle(self) -> Self;
    fn num() -> usize;
    fn transition(self, surround: &[u8]) -> Self;
    fn place_mode(self) -> Self {
        self.toggle()
    }
    fn seed(&self) -> u8 {
        0
    }
}

impl<T: CellState + Send + Sync + 'static> Game<T> {
    pub fn new(width: usize, height: usize) -> Game<T> {
        const GAPSET: i32 = 0;
        const PAD: u32 = 0;

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
            surround: vec![vec![0; T::num() + 1]; num_cpus::get()],
            join_handles: Vec::with_capacity(num_cpus::get()),
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
        self.join_handles.clear();
        let threads = unsafe {
            core::slice::from_raw_parts_mut(self.join_handles.as_mut_ptr(), num_cpus::get())
        };
        let self_ptr: *const Self = self;
        for (t, thread) in threads.iter_mut().enumerate().take(num_cpus::get()) {
            let self_ptr = GamePtr { game: self_ptr };
            *thread = Some(thread::spawn(move || unsafe {
                let self_ptr = self_ptr;

                for idx_x in (t..(*self_ptr.game).cells.len()).step_by(num_cpus::get()) {
                    for idx_y in 0..(*self_ptr.game).cells[0].len() {
                        (*self_ptr.game).update_surround(idx_x as isize, idx_y as isize, t);
                        (*self_ptr.game.cast_mut()).next_cells[idx_x][idx_y].state =
                            (*self_ptr.game).cells[idx_x][idx_y]
                                .state
                                .transition((*self_ptr.game).surround[t].as_slice());
                    }
                }
            }));
        }

        for thread in threads {
            thread.take().and_then(|t| t.join().ok());
        }

        self.cells.swap_with_slice(&mut self.next_cells);
    }

    fn cell_is_state(&self, idx_x: isize, idx_y: isize, state: T) -> bool {
        self.cell(idx_x, idx_y)
            .and_then(|cell| cell.option_state(state))
            .is_some()
    }

    fn update_surround(&self, idx_x: isize, idx_y: isize, thread: usize) {
        let surround = self.surround.as_ptr().cast_mut();
        let surround = unsafe { core::slice::from_raw_parts_mut(surround, self.surround.len()) };
        let surround = &mut surround[thread];
        for v in surround.iter_mut() {
            *v = 0;
        }

        for (i, surround) in surround.iter_mut().enumerate().take(T::num()) {
            *surround += self.cell_is_state(idx_x - 1, idx_y - 1, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x, idx_y - 1, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x + 1, idx_y - 1, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x - 1, idx_y, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x + 1, idx_y, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x - 1, idx_y + 1, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x, idx_y + 1, T::from(i)) as u8;
            *surround += self.cell_is_state(idx_x + 1, idx_y + 1, T::from(i)) as u8;
        }

        surround[T::num()] += (1..T::num()).fold(0, |acc, n| {
            acc + self.cell_is_state(idx_x, idx_y, T::from(n)) as u8 * n as u8
        });
    }

    fn cell(&self, idx_x: isize, idx_y: isize) -> Option<&Cell<T>> {
        let (idx_x, idx_y) = self.wrap_idx(idx_x, idx_y);
        self.cells
            .get(idx_x)
            .and_then(|i: &Vec<Cell<T>>| i.get(idx_y))
    }

    fn wrap_idx(&self, idx_x: isize, idx_y: isize) -> (usize, usize) {
        let x = if idx_x % (self.width as isize) < 0 {
            self.width as isize - 1
        } else {
            idx_x % self.width as isize
        };
        let y = if idx_y % (self.height as isize) < 0 {
            self.height as isize - 1
        } else {
            idx_y % self.height as isize
        };

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

struct CanvasPtr<T: RenderTarget> {
    canvas: *const Canvas<T>,
}

unsafe impl<T: RenderTarget> Send for CanvasPtr<T> { }
unsafe impl<T: RenderTarget> Sync for CanvasPtr<T> { }

pub fn draw_grid<T: CellState + Send + Sync + 'static>(ctx: &mut Ctx<T>) {
    //for t in 0..num_cpus::get() {
    //    let ptr = GamePtr { game: &ctx.game };
    //    let threads = unsafe { core::slice::from_raw_parts_mut(ctx.draw_threads.as_mut_ptr(), num_cpus::get()) };
    //    let cells = unsafe {&(*ptr.game).cells};
    //    let canvas_ptr: CanvasPtr<Window> = CanvasPtr { canvas: &ctx.canvas };
    //    threads[t] = Some(thread::spawn(move || unsafe {
    //        let canvas_ptr = canvas_ptr;
    //        let canvas = &mut (*canvas_ptr.canvas.cast_mut());

    //        for idx_x in (0..cells.len()).step_by(num_cpus::get()) {
    //            for idx_y in 0..cells[0].len() {
    //                let cell = &cells[idx_x][idx_y];
    //                canvas.set_draw_color(cell.color());
    //                canvas.fill_rect(cell.rect).unwrap();
    //            }
    //        }
    //    }));
    //}

    let cells = &ctx.game.cells;
    for i in cells {
        for cell in i {
            ctx.canvas.set_draw_color(cell.color());
            ctx.canvas.fill_rect(cell.rect).unwrap();
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
        game: Game::<Gol>::new(BOARD_WIDTH, BOARD_HEIGHT),
        draw_threads: Vec::with_capacity(num_cpus::get()),
    };

    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut autoplay = false;
    let mut autoplay_speed = 5;
    let mut autoplay_acc = 0;

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
                    keycode: Some(Keycode::P),
                    ..
                } => {
                    autoplay = !autoplay;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Equals),
                    ..
                } => {
                    autoplay_speed -= 1;
                    autoplay_speed = autoplay_speed.clamp(1, 30);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Minus),
                    ..
                } => {
                    autoplay_speed += 1;
                        autoplay_speed = autoplay_speed.clamp(1, 30);
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

        if autoplay {
            if autoplay_acc == 0 {
                ctx.game.next_state();
            }
            autoplay_acc = (autoplay_acc + 1) % autoplay_speed;
        }

        draw_grid(&mut ctx);
        ctx.canvas.present();
        thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS));
    }
}
