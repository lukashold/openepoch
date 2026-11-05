//! Source 004E5CB0. The dispatcher owns interval and cursor progression.
//! The caller supplies the source time/configuration and the separately
//! resolved engine callbacks. Result codes and the shared result byte retain
//! their numeric form until the check and submit functions are understood.
use crate::ai9_count::Precision53;
use crate::ai9_planner::{Name, PlannerData};
use crate::ai9_tech_tree::{self, TechnologyTree};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResearchState {
    /// Planner field +4C, written when the periodic check is due.
    pub last_check: f32,
    /// Planner field +68, written after a particular list submission result.
    pub last_list_submit: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Timing {
    /// Source game time stays constant for this dispatch.
    pub now: f32,
    pub check_interval: f32,
    /// Runtime integer at C08E30.
    pub epoch_delay: i32,
    /// Runtime integer at C08E2C.
    pub measure_threshold: i32,
    pub precision: Precision53,
}

pub trait ResearchQueries {
    fn mode(&mut self) -> i32;
    /// 004E5FE0, dispatched for the source's mode 1 and mode 2 conditions.
    fn alternate_mode(&mut self);
    /// 006E2370 lookup followed by 004E86B0 when a row exists.
    fn status8_measure(&mut self, name: &Name) -> Option<i32>;
    /// 006E2610, with -1 represented as None, then 006E24B0.
    fn epoch_alternative(&mut self) -> Option<Name>;
    /// Capture 006E25F0 for the current 005E0070 epoch.
    fn begin_epoch_fallback(&mut self);
    fn fallback_count(&self) -> usize;
    fn fallback_category(&mut self, index: usize) -> i32;
    fn fallback_name(&mut self, index: usize) -> Name;
}

pub trait ResearchHost: ResearchQueries {
    /// 004E7B80(name, 0, 0, status, 0). The initial stack slot has no value.
    fn check(&mut self, name: &Name, status: &mut Option<i32>) -> bool;
    /// 004E7EA0(name, flag). All submissions in a pass share the flag byte.
    fn submit(&mut self, name: &Name, flag: &mut u8) -> i32;
}

impl<T: ResearchQueries + TechnologyTree> ResearchHost for T {
    fn check(&mut self, name: &Name, status: &mut Option<i32>) -> bool {
        ai9_tech_tree::check(self, name, status)
    }

    fn submit(&mut self, name: &Name, flag: &mut u8) -> i32 {
        ai9_tech_tree::submit(self, name, flag)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    NotDue,
    AlternateMode,
    ListExhausted,
    ListStopped(i32),
    EpochFallback { stop_code: Option<i32> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvalidInput {
    NonFiniteTime,
    MissingCheckStatus,
}

pub fn dispatch(
    state: &mut ResearchState,
    data: &mut PlannerData,
    timing: Timing,
    host: &mut impl ResearchHost,
) -> Result<Pass, InvalidInput> {
    if [
        state.last_check,
        state.last_list_submit,
        timing.now,
        timing.check_interval,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(InvalidInput::NonFiniteTime);
    }
    let now = f64::from(timing.now);
    let next_check = timing.precision.add(
        f64::from(timing.check_interval),
        f64::from(state.last_check),
    );
    if next_check > now {
        return Ok(Pass::NotDue);
    }
    state.last_check = timing.now;
    if host.mode() == 1 || host.mode() == 2 {
        host.alternate_mode();
        return Ok(Pass::AlternateMode);
    }

    data.rewind_technologies();
    let epoch_due = timing.precision.add(
        f64::from(timing.epoch_delay),
        f64::from(state.last_list_submit),
    ) <= now;
    let mut name = None;
    let mut status = None;
    let mut flag = 0;
    while data.next_technology(&mut name) {
        let candidate = name
            .as_ref()
            .expect("The reset iterator addresses an existing name");
        let (mut code, mut submitted) = attempt(host, candidate, &mut status, &mut flag)?;
        let mut measure = 0;
        if !submitted && code == 8 {
            let Some(found) = host.status8_measure(candidate) else {
                continue;
            };
            measure = found;
            if epoch_due || measure > timing.measure_threshold {
                if let Some(alternative) = host.epoch_alternative() {
                    (code, submitted) = attempt(host, &alternative, &mut status, &mut flag)?;
                }
            }
        }

        if code == 0 && flag != 0 && submitted {
            state.last_list_submit = timing.now;
        }
        if code == 2 || (code == 0 && flag == 0) {
            continue;
        }
        if code == 5 || (code == 8 && (epoch_due || measure > timing.measure_threshold)) {
            return epoch_fallback(host, &mut status, &mut flag);
        }
        return Ok(Pass::ListStopped(code));
    }
    Ok(Pass::ListExhausted)
}

fn attempt(
    host: &mut impl ResearchHost,
    name: &Name,
    status: &mut Option<i32>,
    flag: &mut u8,
) -> Result<(i32, bool), InvalidInput> {
    if host.check(name, status) {
        let code = host.submit(name, flag);
        *status = Some(code);
        return Ok((code, true));
    }
    status
        .map(|code| (code, false))
        .ok_or(InvalidInput::MissingCheckStatus)
}

fn epoch_fallback(
    host: &mut impl ResearchHost,
    status: &mut Option<i32>,
    flag: &mut u8,
) -> Result<Pass, InvalidInput> {
    host.begin_epoch_fallback();
    let mut index = 0;
    while index < host.fallback_count() {
        if (2..=4).contains(&host.fallback_category(index)) {
            let name = host.fallback_name(index);
            let (code, _) = attempt(host, &name, status, flag)?;
            if code != 2 && code != 8 && (code != 0 || *flag != 0) {
                return Ok(Pass::EpochFallback {
                    stop_code: Some(code),
                });
            }
        }
        index += 1;
    }
    Ok(Pass::EpochFallback { stop_code: None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai9_planner::{AipRead, BuildRow, Context};
    use std::collections::VecDeque;

    struct TechnologyList(Vec<Name>);

    impl AipRead for TechnologyList {
        fn registry(&self, _: Context) -> &[Name] {
            &[]
        }
        fn build_rows(&self, _: &Name) -> &[BuildRow] {
            &[]
        }
        fn tech_names(&self) -> &[Name] {
            &self.0
        }
    }

    #[derive(Debug)]
    enum Event {
        Mode(i32),
        AlternateMode,
        Check(&'static str, bool, Option<i32>),
        Submit(&'static str, u8, u8, i32),
        Measure(&'static str, Option<i32>),
        Alternative(Option<&'static str>),
        BeginFallback,
        Category(usize, i32),
        FallbackName(usize, &'static str),
    }

    struct Host {
        expected: VecDeque<Event>,
        fallback_count: usize,
    }

    impl Host {
        fn new(events: impl IntoIterator<Item = Event>) -> Self {
            Self {
                expected: events.into_iter().collect(),
                fallback_count: 0,
            }
        }

        fn next(&mut self) -> Event {
            self.expected
                .pop_front()
                .expect("Unexpected engine callback")
        }

        fn verify(&self) {
            assert!(
                self.expected.is_empty(),
                "Unconsumed callbacks: {:?}",
                self.expected
            );
        }
    }

    impl ResearchQueries for Host {
        fn mode(&mut self) -> i32 {
            let Event::Mode(value) = self.next() else {
                panic!("Expected mode callback");
            };
            value
        }

        fn alternate_mode(&mut self) {
            assert!(matches!(self.next(), Event::AlternateMode));
        }

        fn status8_measure(&mut self, name: &Name) -> Option<i32> {
            let Event::Measure(expected, value) = self.next() else {
                panic!("Expected measure callback");
            };
            assert_eq!(name.as_str(), expected);
            value
        }

        fn epoch_alternative(&mut self) -> Option<Name> {
            let Event::Alternative(value) = self.next() else {
                panic!("Expected alternative callback");
            };
            value.map(|name| Name::new(name).unwrap())
        }

        fn begin_epoch_fallback(&mut self) {
            assert!(matches!(self.next(), Event::BeginFallback));
        }

        fn fallback_count(&self) -> usize {
            self.fallback_count
        }

        fn fallback_category(&mut self, index: usize) -> i32 {
            let Event::Category(expected, value) = self.next() else {
                panic!("Expected category callback");
            };
            assert_eq!(index, expected);
            value
        }

        fn fallback_name(&mut self, index: usize) -> Name {
            let Event::FallbackName(expected, value) = self.next() else {
                panic!("Expected fallback name callback");
            };
            assert_eq!(index, expected);
            Name::new(value).unwrap()
        }
    }

    // These traces exercise the caller's raw branches independently of the
    // result domain provided by the real 004E7B80 and 004E7EA0 leaves.
    impl ResearchHost for Host {
        fn check(&mut self, name: &Name, status: &mut Option<i32>) -> bool {
            let Event::Check(expected, ready, value) = self.next() else {
                panic!("Expected check callback");
            };
            assert_eq!(name.as_str(), expected);
            if value.is_some() {
                *status = value;
            }
            ready
        }

        fn submit(&mut self, name: &Name, flag: &mut u8) -> i32 {
            let Event::Submit(expected, before, after, code) = self.next() else {
                panic!("Expected submit callback");
            };
            assert_eq!(name.as_str(), expected);
            assert_eq!(*flag, before);
            *flag = after;
            code
        }
    }

    fn setup(names: &[&str]) -> (PlannerData, ResearchState, Timing) {
        let mut data = PlannerData::default();
        data.apply(&TechnologyList(
            names.iter().map(|name| Name::new(name).unwrap()).collect(),
        ));
        let state = ResearchState {
            last_check: 45.0,
            last_list_submit: 10.0,
        };
        let timing = Timing {
            now: 55.0,
            check_interval: 10.0,
            epoch_delay: 50,
            measure_threshold: 20,
            precision: Precision53::NearestEven,
        };
        (data, state, timing)
    }

    #[test]
    fn interval_compares_before_f32_rounding_and_mode_branch_skips_rewind() {
        use Event::*;
        let (mut data, mut state, mut timing) = setup(&["A", "B"]);
        assert!(data.next_technology(&mut None));
        timing.now = 16_777_216.0;
        timing.check_interval = 0.75;
        state.last_check = timing.now;
        assert_eq!(
            dispatch(&mut state, &mut data, timing, &mut Host::new([])),
            Ok(Pass::NotDue)
        );
        assert_eq!(data.tech_cursor(), 0);
        timing.check_interval = 0.0;
        for events in [
            vec![Mode(1), AlternateMode],
            vec![Mode(0), Mode(2), AlternateMode],
        ] {
            let mut host = Host::new(events);
            assert_eq!(
                dispatch(&mut state, &mut data, timing, &mut host),
                Ok(Pass::AlternateMode)
            );
            assert_eq!(data.tech_cursor(), 0);
            host.verify();
        }
    }

    #[test]
    fn result_byte_survives_code_two_and_check_only_stop_keeps_submit_time() {
        use Event::*;
        let (mut data, mut state, timing) = setup(&["A", "B", "C"]);
        assert!(data.next_technology(&mut None));
        let mut host = Host::new([
            Mode(0),
            Mode(0),
            Check("A", true, None),
            Submit("A", 0, 7, 2),
            Check("B", false, Some(0)),
        ]);
        assert_eq!(
            dispatch(&mut state, &mut data, timing, &mut host),
            Ok(Pass::ListStopped(0))
        );
        assert_eq!(data.tech_cursor(), 1);
        assert_eq!(
            state,
            ResearchState {
                last_check: 55.0,
                last_list_submit: 10.0
            }
        );
        host.verify();
    }

    #[test]
    fn status_eight_missing_row_skips_and_strict_measure_opens_alternative() {
        use Event::*;
        for measure in [20, 21] {
            let (mut data, mut state, timing) = setup(&["missing", "threshold", "unused"]);
            let mut events = vec![
                Mode(0),
                Mode(0),
                Check("missing", false, Some(8)),
                Measure("missing", None),
                Check("threshold", false, Some(8)),
                Measure("threshold", Some(measure)),
            ];
            if measure == 21 {
                events.extend([
                    Alternative(Some("Main")),
                    Check("Main", true, None),
                    Submit("Main", 0, 1, 0),
                ]);
            }
            let mut host = Host::new(events);
            let expected_code = if measure == 20 { 8 } else { 0 };
            assert_eq!(
                dispatch(&mut state, &mut data, timing, &mut host),
                Ok(Pass::ListStopped(expected_code))
            );
            assert_eq!(
                state.last_list_submit,
                if measure == 20 { 10.0 } else { 55.0 }
            );
            assert_eq!(data.tech_cursor(), 1);
            host.verify();
        }
    }

    #[test]
    fn epoch_delay_equality_opens_alternative_before_fallback() {
        use Event::*;
        let (mut data, mut state, mut timing) = setup(&["A"]);
        timing.now = 60.0;
        let mut host = Host::new([
            Mode(0),
            Mode(0),
            Check("A", false, Some(8)),
            Measure("A", Some(0)),
            Alternative(None),
            BeginFallback,
        ]);
        assert_eq!(
            dispatch(&mut state, &mut data, timing, &mut host),
            Ok(Pass::EpochFallback { stop_code: None })
        );
        assert_eq!(state.last_list_submit, 10.0);
        host.verify();
    }

    #[test]
    fn fallback_filters_categories_and_keeps_list_submit_timestamp() {
        use Event::*;
        let (mut data, mut state, timing) = setup(&["A", "unused"]);
        let mut host = Host::new([
            Mode(0),
            Mode(0),
            Check("A", false, Some(5)),
            BeginFallback,
            Category(0, 1),
            Category(1, 2),
            FallbackName(1, "F1"),
            Check("F1", true, None),
            Submit("F1", 0, 3, 2),
            Category(2, 4),
            FallbackName(2, "F2"),
            Check("F2", false, Some(8)),
            Category(3, 3),
            FallbackName(3, "F3"),
            Check("F3", true, None),
            Submit("F3", 3, 4, 0),
        ]);
        host.fallback_count = 6;
        assert_eq!(
            dispatch(&mut state, &mut data, timing, &mut host),
            Ok(Pass::EpochFallback { stop_code: Some(0) })
        );
        assert_eq!(state.last_list_submit, 10.0);
        assert_eq!(data.tech_cursor(), 0);
        host.verify();
    }
}
