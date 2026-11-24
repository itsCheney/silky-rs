//! The flag list's view of now: before it shows a waiting event, the
//! original walks the hero from where he is to the event's place, adding
//! the minutes each step takes, and moves an event whose window is open to
//! when he would get there. One clock serves all of a girl's events in
//! turn, so a walk to one event is where the next one starts from.

use super::flaglist::Record;

/// The last day of the story.
const LAST_DAY: i32 = 830;
/// The day's first and last hour of walking about.
const MORNING: i32 = 830;
const EVENING: i32 = 1800;
/// The first hour a trip to another town can leave.
const TRIP_EARLIEST: i32 = 500;
/// What a trip to or from the stage 1 town costs, in hundreds of yen.
const TRIP_COST: i32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Clock {
    /// Stage (0 the town, 1 the other town, 2 to 5 the school floors) and
    /// the place within it.
    pub stage: i32,
    pub sub: i32,
    /// Hours × 100 + minutes, and month × 100 + day.
    pub time: i32,
    pub date: i32,
    /// In hundreds of yen.
    pub cash: i32,
}

impl Clock {
    fn add(&mut self, minutes: i32) {
        let mut h = self.time / 100;
        let mut m = minutes + self.time % 100;
        if m > 59 {
            h += m / 60;
            m %= 60;
        }
        if h > 23 {
            self.date += h / 24;
            h %= 24;
        }
        self.time = h * 100 + m;
    }

    /// Too late in the day: the next morning.
    fn next_morning_after(&mut self, last: i32) {
        if self.time > last {
            self.date += 1;
            self.time = MORNING;
        }
    }

    /// Not before the morning, and the next morning when too late.
    fn within_day(&mut self, last: i32) {
        if self.time < MORNING {
            self.time = MORNING;
        } else {
            self.next_morning_after(last);
        }
    }

    /// A trip leaves no earlier than five in the morning.
    fn trip_hour(&mut self) {
        if (1..TRIP_EARLIEST).contains(&self.time) {
            self.time = TRIP_EARLIEST;
        }
    }

    /// Walks to `stage` and `sub`: whether that ends past the last day.
    pub fn walk(&mut self, stage: i32, sub: i32) -> bool {
        if self.stage == stage && self.sub == sub && sub != 0 {
            match (self.stage, self.sub) {
                (0, 3) => {
                    let late = self.time < MORNING || self.time >= EVENING;
                    self.add(if late { 15 } else { 10 });
                }
                (0, 4) => {
                    self.add(10);
                    self.within_day(EVENING - 1);
                }
                (0, 1) => self.add(15),
                (1, _) => {
                    self.add(15);
                    self.sub = 2;
                }
                _ => {}
            }
            if self.date > LAST_DAY {
                return true;
            }
        }
        while self.stage != stage {
            match self.stage {
                0 if stage == 1 => {
                    if self.sub != 1 {
                        self.add(15);
                    }
                    self.trip_hour();
                    self.add(30);
                    self.stage = 1;
                    self.sub = 2;
                }
                0 => {
                    match self.sub {
                        0 | 1 => self.add(15),
                        4 => self.add(10),
                        _ => {}
                    }
                    self.within_day(EVENING);
                    self.stage = 2;
                    self.sub = 0;
                }
                1 => {
                    if self.sub != 2 {
                        self.add(15);
                    }
                    self.trip_hour();
                    self.add(30);
                    self.stage = 0;
                    self.sub = 1;
                }
                2 => {
                    self.add(10);
                    self.next_morning_after(EVENING);
                    if stage < 2 {
                        self.stage = 0;
                        self.sub = 3;
                    } else {
                        self.stage = 3;
                        self.sub = 0;
                    }
                }
                3..=5 => {
                    self.sub = 0;
                    self.add(10);
                    self.next_morning_after(EVENING);
                    if stage > self.stage {
                        self.stage += 1;
                    } else {
                        self.stage -= 1;
                    }
                }
                _ => return false,
            }
        }
        if sub == 0 && self.sub == 0 {
            self.add(if self.stage < 2 { 15 } else { 10 });
        }
        loop {
            if self.sub == sub {
                return self.date > LAST_DAY;
            }
            match self.sub {
                3 if sub == 4 => {
                    self.within_day(EVENING - 1);
                    self.sub = 4;
                }
                3 | 0 => {
                    self.add(15);
                    self.sub = sub;
                }
                4 => {
                    self.add(10);
                    self.sub = 3;
                }
                1 => {
                    self.add(15);
                    self.sub = if sub < 3 { 0 } else { 3 };
                }
                2 => {
                    self.add(15);
                    self.sub = 0;
                }
                _ => return self.date > LAST_DAY,
            }
        }
    }

    /// Fits the waiting event `r` to now: what the trip there costs, and
    /// for an event whose window is open, the day and time the hero would
    /// get there. Whether the event is out of reach (`reason` set when it
    /// is the cash).
    pub fn fit(&mut self, r: &mut Record, no_cash: u16) -> bool {
        r.cost = if self.stage != r.stage && (self.stage == 1 || r.stage == 1) {
            TRIP_COST
        } else {
            0
        };
        if self.cash < r.cost {
            r.reason = no_cash;
            return true;
        }
        if r.d0 > LAST_DAY {
            r.d0 = LAST_DAY;
            return true;
        }
        if self.date < r.d0 {
            return false;
        }
        if (r.d1 != -1 && r.d1 < r.d0) || self.walk(r.stage, r.sub) {
            return true;
        }
        let (day, now) = (self.date, self.time);
        if r.d1 == -1 && r.d0 == day {
            if now < r.t0 || (r.t1 != -1 && r.t0 == now) {
                return false;
            }
            if r.t1 < now {
                return true;
            }
            r.t0 = now;
            return false;
        }
        if day < r.d0 || r.d1 < day {
            return true;
        }
        r.d0 = day;
        if now < r.t0 {
            return false;
        }
        if r.t1 != -1 && now <= r.t1 {
            r.t0 = now;
            return false;
        }
        if day + 1 <= r.d1 {
            r.d0 = day + 1;
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(stage: i32, sub: i32, time: i32) -> Clock {
        Clock {
            stage,
            sub,
            time,
            date: 815,
            cash: 0,
        }
    }

    #[test]
    fn minutes_roll_over() {
        let mut c = at(0, 0, 2350);
        c.add(15);
        assert_eq!((c.date, c.time), (816, 5));
    }

    #[test]
    fn walks() {
        // From the town to the school's second floor: to the gate, then a
        // floor at a time.
        let mut c = at(0, 0, 1000);
        assert!(!c.walk(3, 0));
        assert_eq!((c.stage, c.sub, c.time), (3, 0, 1035));
        // A trip waits for five in the morning and takes half an hour.
        let mut c = at(0, 1, 200);
        assert!(!c.walk(1, 2));
        assert_eq!((c.stage, c.sub, c.time), (1, 2, 530));
        // Past the last day.
        let mut c = at(0, 0, 1900);
        c.date = 830;
        assert!(c.walk(2, 0));
    }
}
