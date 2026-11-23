//! The screen: the dice to pick from, then the roll and the ones before it.

use xpui::{Button, CellGrid, Modifiers, NavigationScreen, Renderer, Screen, Text, View, vstack};

use crate::art::paint_die;
use crate::dice::{Dice, SIDES};
use crate::readout::Readout;

/// The die picked first: the d20.
const FIRST: usize = 5;

#[derive(Clone, Copy)]
pub enum Msg {
    /// A die was tapped: pick it and roll it.
    Pick(usize),
    /// A key moved to a die.
    Select(usize),
    Roll,
}

pub struct DiceScreen {
    dice: Dice,
}

impl DiceScreen {
    pub fn new() -> Self {
        DiceScreen {
            dice: Dice::new(FIRST),
        }
    }
}

impl Screen for DiceScreen {
    type Message = Msg;

    fn title(&self) -> Option<&'static str> {
        Some("Dice")
    }

    fn body(&self) -> impl View<Msg> {
        let columns = if Renderer::screen_size().width >= 700 {
            7
        } else {
            4
        };
        let dice_count = SIDES.len() as i32;
        let rows = (dice_count + columns - 1) / columns;
        let selected = self.dice.selected;
        let dice = CellGrid::new(columns as usize, rows as usize, move |index, cell| {
            paint_die(index, cell, index == selected);
        })
        .on_cell_tap(Msg::Pick);
        let mut body = vstack![12; dice];
        if self.dice.rolled == 0 {
            body = body.push(Text::new("No rolls yet. Tap a die to roll it."));
        } else {
            body = body
                .push(Text::new(self.dice.verdict()).on_tap(Msg::Roll))
                .push(Readout::new(self.dice.rolled).flexible())
                .push(Text::new(self.dice.history()));
        }
        NavigationScreen::new(body)
    }

    fn on_key(&self, key: Button) -> Option<Msg> {
        let (selected, count) = (self.dice.selected, SIDES.len());
        match key {
            Button::Confirm => Some(Msg::Roll),
            Button::Up | Button::Left => Some(Msg::Select((selected + count - 1) % count)),
            Button::Down | Button::Right => Some(Msg::Select((selected + 1) % count)),
            _ => None,
        }
    }

    fn update(&mut self, message: Msg) {
        match message {
            Msg::Pick(index) if index < SIDES.len() => {
                self.dice.selected = index;
                self.dice.roll();
            }
            Msg::Pick(_) => {}
            Msg::Select(index) => self.dice.select(index),
            Msg::Roll => self.dice.roll(),
        }
    }
}
