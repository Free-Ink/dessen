use crate::dice::{Dice, NAMES, SIDES};
use dessen::reset;

fn roll_with(dice: &mut Dice, random: u32) {
    dessen::with(|w| w.randoms.push(random));
    dice.roll();
}

#[test]
fn a_roll_spans_one_to_the_sides_without_bias() {
    reset(|_| {});
    for (index, sides) in SIDES.iter().enumerate() {
        let mut dice = Dice::new(index);
        roll_with(&mut dice, 0);
        assert_eq!(dice.rolled, 1, "{}", NAMES[index]);
        roll_with(&mut dice, u32::MAX);
        assert_eq!(dice.rolled, *sides, "{}", NAMES[index]);
        roll_with(&mut dice, 1 << 31);
        assert_eq!(dice.rolled, sides / 2 + 1, "{}: the middle", NAMES[index]);
    }
}

#[test]
fn many_rolls_stay_on_the_die() {
    reset(|w| w.seed = 12345);
    let mut dice = Dice::new(6); // d100
    let mut seen = [false; 101];
    for _ in 0..5000 {
        dice.roll();
        assert!((1..=100).contains(&dice.rolled));
        seen[dice.rolled as usize] = true;
    }
    assert!(seen[1..].iter().all(|s| *s), "every face comes up");
}

#[test]
fn only_a_d20_is_critical() {
    reset(|_| {});
    let mut d20 = Dice::new(5);
    roll_with(&mut d20, u32::MAX);
    assert_eq!(d20.verdict(), "Rolled d20: critical hit!");
    roll_with(&mut d20, 0);
    assert_eq!(d20.verdict(), "Rolled d20: critical fail.");
    roll_with(&mut d20, 1 << 31);
    assert_eq!(d20.verdict(), "Rolled d20");
    let mut d4 = Dice::new(0);
    roll_with(&mut d4, 0);
    assert_eq!(d4.verdict(), "Rolled d4");
}

#[test]
fn history_keeps_the_last_five_newest_first() {
    reset(|_| {});
    let mut dice = Dice::new(1); // d6
    assert_eq!(dice.history(), "Last:");
    for face in 1..=6u32 {
        // The random number that rolls `face` on a d6.
        roll_with(&mut dice, (((face - 1) as u64 * (1u64 << 32)) / 6 + 1) as u32);
        assert_eq!(dice.rolled, face);
    }
    assert_eq!(dice.history(), "Last:  d6=6  d6=5  d6=4  d6=3  d6=2");
    dice.select(5);
    assert_eq!(dice.rolled, 0, "picking a die clears the last roll");
    roll_with(&mut dice, 0);
    assert_eq!(dice.history(), "Last:  d20=1  d6=6  d6=5  d6=4  d6=3");
}
