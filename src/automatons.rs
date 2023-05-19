#![allow(dead_code)]
use sdl2::pixels::Color;

use crate::CellState;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum Seeds {
    #[default]
    White,
    Gray,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum Gol {
    #[default]
    White,
    Gray,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum BB {
    #[default]
    White,
    Red,
    Gray,
}

impl CellState for BB {
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

    fn transition(self, surround: &[u8]) -> Self {
        match surround {
            [_, _, 2, 0] => Self::Gray,
            [_, _, _, 2] => Self::Red,
            [_, _, _, 1] => Self::White,
            _ => Self::White,
        }
    }
}

impl From<usize> for BB {
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

    fn transition(self, surround: &[u8]) -> Self {
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

impl CellState for Gol {
    fn color(&self) -> Color {
        match self {
            Gol::White => Color::WHITE,
            Gol::Gray => Color::GRAY,
        }
    }

    fn toggle(self) -> Self {
        match self {
            Gol::White => Gol::Gray,
            Gol::Gray => Gol::White,
        }
    }

    fn num() -> usize {
        2
    }

    fn transition(self, surround: &[u8]) -> Self {
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

impl From<usize> for Gol {
    fn from(n: usize) -> Self {
        match n {
            0 => Gol::White,
            1 => Gol::Gray,
            _ => panic!("Out of bounds {n}"),
        }
    }
}
