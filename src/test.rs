mod component;

use crate::*;

macro_rules! logic_state {
    ($state:ident) => {
        LogicState::$state
    };
    ({% $($bit:tt),*}) => {
        $crate::bits!($($bit),*)
    };
    ({$value:expr}) => {
        LogicState::from_int($value)
    };
    ([$($value:expr),+ $(,)?]) => {
        LogicState::from_big_int([$($value),+].as_slice()).unwrap()
    };
    ($value:expr) => {
        LogicState::from_int($value)
    };
}

use logic_state;

struct BinaryGateTestData {
    input_a: LogicState,
    input_b: LogicState,
    output: LogicState,
}

fn test_binary_gate<F>(add_gate: F, width: u32, test_data: &[BinaryGateTestData], max_steps: u64)
where
    F: FnOnce(&mut SimulatorBuilder, WireId, WireId, WireId) -> AddComponentResult,
{
    let mut builder = SimulatorBuilder::default();
    let input_wire_a = builder.add_wire(width).unwrap();
    let input_wire_b = builder.add_wire(width).unwrap();
    let output_wire = builder.add_wire(width).unwrap();
    let _gate = add_gate(&mut builder, input_wire_a, input_wire_b, output_wire).unwrap();

    let mut sim = builder.build().unwrap();

    for (i, test_data) in test_data.iter().enumerate() {
        sim.reset();

        sim.set_wire_drive(input_wire_a, &test_data.input_a)
            .unwrap();
        sim.set_wire_drive(input_wire_b, &test_data.input_b)
            .unwrap();

        match sim.run(max_steps) {
            SimulationRunResult::Ok => {}
            SimulationRunResult::MaxStepsReached => panic!("[TEST {i}] exceeded max steps"),
            SimulationRunResult::Err { conflicting_wires } => {
                panic!("[TEST {i}] wire conflicts: {conflicting_wires:?}");
            }
        }

        let output_state = sim.get_wire_state(output_wire).unwrap();

        assert!(
            output_state.eq(&test_data.output, width),
            "[TEST {i}]  expected: {}  actual: {}",
            test_data.output.to_string(width),
            output_state.to_string(width),
        );
    }
}

//fn test_shifter<F>(add_gate: F, width: NonZeroU8, test_data: &[BinaryGateTestData], max_steps: u64)
//where
//    F: FnOnce(&mut SimulatorBuilder, WireId, WireId, WireId) -> AddComponentResult,
//{
//    let mut builder = SimulatorBuilder::default();
//
//    let shamnt_width = NonZeroU8::new(width.clog2()).unwrap();
//    let input_a = builder.add_wire(width).unwrap();
//    let input_b = builder.add_wire(shamnt_width).unwrap();
//    let output = builder.add_wire(width).unwrap();
//    let _gate = add_gate(&mut builder, input_a, input_b, output).unwrap();
//
//    let mut sim = builder.build();
//
//    for (i, test_data) in test_data.iter().enumerate() {
//        sim.set_wire_drive(input_a, &test_data.input_a).unwrap();
//        sim.set_wire_drive(input_b, &test_data.input_b).unwrap();
//
//        match sim.run_sim(max_steps) {
//            SimulationRunResult::Ok => {}
//            SimulationRunResult::MaxStepsReached => panic!("[TEST {i}] exceeded max steps"),
//            SimulationRunResult::Err(err) => panic!("[TEST {i}] {err:?}"),
//        }
//
//        let output_state = sim.get_wire_state(output).unwrap();
//
//        assert!(
//            output_state.eq(&test_data.output, width),
//            "[TEST {i}]  expected: {}  actual: {}",
//            test_data.output.display_string(width),
//            output_state.display_string(width),
//        );
//    }
//}
//
//fn test_binary_module(
//    sim: &mut Simulator,
//    input_a: WireId,
//    input_b: WireId,
//    output: WireId,
//    width: NonZeroU8,
//    test_data: &[BinaryGateTestData],
//    max_steps: u64,
//) {
//    for (i, test_data) in test_data.iter().enumerate() {
//        sim.set_wire_drive(input_a, &test_data.input_a).unwrap();
//        sim.set_wire_drive(input_b, &test_data.input_b).unwrap();
//
//        match sim.run_sim(max_steps) {
//            SimulationRunResult::Ok => {}
//            SimulationRunResult::MaxStepsReached => panic!("[TEST {i}] exceeded max steps"),
//            SimulationRunResult::Err(err) => panic!("[TEST {i}] {err:?}"),
//        }
//
//        let output_state = sim.get_wire_state(output).unwrap();
//
//        assert!(
//            output_state.eq(&test_data.output, width),
//            "[TEST {i}]  expected: {}  actual: {}",
//            test_data.output.display_string(width),
//            output_state.display_string(width),
//        );
//    }
//}

macro_rules! binary_gate_test_data {
    ($(($a:tt, $b:tt) -> $o:tt),* $(,)?) => {
        &[
            $(
                BinaryGateTestData {
                    input_a: logic_state!($a),
                    input_b: logic_state!($b),
                    output: logic_state!($o),
                },
            )*
        ]
    };
}

use binary_gate_test_data;

struct UnaryGateTestData {
    input: LogicState,
    output: LogicState,
}

fn test_unary_gate<F>(add_gate: F, width: u32, test_data: &[UnaryGateTestData], max_steps: u64)
where
    F: FnOnce(&mut SimulatorBuilder, WireId, WireId) -> AddComponentResult,
{
    let mut builder = SimulatorBuilder::default();
    let input_wire = builder.add_wire(width).unwrap();
    let output_wire = builder.add_wire(width).unwrap();
    let _gate = add_gate(&mut builder, input_wire, output_wire).unwrap();

    let mut sim = builder.build().unwrap();

    for (i, test_data) in test_data.iter().enumerate() {
        sim.reset();

        sim.set_wire_drive(input_wire, &test_data.input).unwrap();

        match sim.run(max_steps) {
            SimulationRunResult::Ok => {}
            SimulationRunResult::MaxStepsReached => panic!("[TEST {i}] exceeded max steps"),
            SimulationRunResult::Err { conflicting_wires } => {
                panic!("[TEST {i}] wire conflicts: {conflicting_wires:?}");
            }
        }

        let output_state = sim.get_wire_state(output_wire).unwrap();

        assert!(
            output_state.eq(&test_data.output, width),
            "[TEST {i}]  expected: {}  actual: {}",
            test_data.output.to_string(width),
            output_state.to_string(width),
        );
    }
}

//fn test_horizontal_gate<F>(
//    add_gate: F,
//    width: NonZeroU8,
//    test_data: &[UnaryGateTestData],
//    max_steps: u64,
//) where
//    F: FnOnce(&mut SimulatorBuilder, WireId, WireId) -> AddComponentResult,
//{
//    let mut builder = SimulatorBuilder::default();
//
//    let input = builder.add_wire(width).unwrap();
//    let output = builder.add_wire(NonZeroU8::MIN).unwrap();
//    let _gate = add_gate(&mut builder, input, output).unwrap();
//
//    let mut sim = builder.build();
//
//    for (i, test_data) in test_data.iter().enumerate() {
//        sim.set_wire_drive(input, &test_data.input).unwrap();
//
//        match sim.run_sim(max_steps) {
//            SimulationRunResult::Ok => {}
//            SimulationRunResult::MaxStepsReached => panic!("[TEST {i}] exceeded max steps"),
//            SimulationRunResult::Err(err) => panic!("[TEST {i}] {err:?}"),
//        }
//
//        let output_state = sim.get_wire_state(output).unwrap();
//
//        assert!(
//            output_state.eq(&test_data.output, NonZeroU8::MIN),
//            "[TEST {i}]  expected: {}  actual: {}",
//            test_data.output.display_string(NonZeroU8::MIN),
//            output_state.display_string(NonZeroU8::MIN),
//        );
//    }
//}

macro_rules! unary_gate_test_data {
    ($($i:tt -> $o:tt),* $(,)?) => {
        &[
            $(
                UnaryGateTestData {
                    input: logic_state!($i),
                    output: logic_state!($o),
                },
            )*
        ]
    };
}

use unary_gate_test_data;

struct WideGateTestData<const N: usize> {
    inputs: [LogicState; N],
    output: LogicState,
}

fn test_wide_gate<F, const N: usize>(
    add_gate: F,
    width: u32,
    test_data: &[WideGateTestData<N>],
    max_steps: u64,
) where
    F: Fn(&mut SimulatorBuilder, &[WireId], WireId) -> AddComponentResult,
{
    let mut builder = SimulatorBuilder::default();
    let input_wires: Vec<_> = (0..N).map(|_| builder.add_wire(width).unwrap()).collect();
    let output_wire = builder.add_wire(width).unwrap();
    let _gate = add_gate(&mut builder, &input_wires, output_wire).unwrap();

    let mut sim = builder.build().unwrap();

    for (i, test_data) in test_data.iter().enumerate() {
        sim.reset();

        for (&input_wire, input) in input_wires.iter().zip(&test_data.inputs) {
            sim.set_wire_drive(input_wire, input).unwrap();
        }

        match sim.run(max_steps) {
            SimulationRunResult::Ok => {}
            SimulationRunResult::MaxStepsReached => panic!("[TEST {i}] exceeded max steps"),
            SimulationRunResult::Err { conflicting_wires } => {
                panic!("[TEST {i}] wire conflicts: {conflicting_wires:?}");
            }
        }

        let output_state = sim.get_wire_state(output_wire).unwrap();

        assert!(
            output_state.eq(&test_data.output, width),
            "[TEST {i}]  expected: {}  actual: {}",
            test_data.output.to_string(width),
            output_state.to_string(width),
        );
    }
}

macro_rules! wide_gate_test_data {
    ($(($($i:tt),+) -> $o:tt),* $(,)?) => {
        &[
            $(
                WideGateTestData {
                    inputs: [$(logic_state!($i)),+],
                    output: logic_state!($o),
                },
            )*
        ]
    };
}

use wide_gate_test_data;

//fn test_comparator<F>(add_comparator: F, compare_op: impl Fn(u32, u32) -> bool)
//where
//    F: Fn(&mut SimulatorBuilder, WireId, WireId, WireId) -> AddComponentResult,
//{
//    const WIDTH: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(4) };
//
//    let mut builder = SimulatorBuilder::default();
//
//    let input_a = builder.add_wire(WIDTH).unwrap();
//    let input_b = builder.add_wire(WIDTH).unwrap();
//    let output = builder.add_wire(NonZeroU8::MIN).unwrap();
//    let _comparator = add_comparator(&mut builder, input_a, input_b, output).unwrap();
//
//    let mut sim = builder.build();
//
//    for a in 0..16 {
//        for b in 0..16 {
//            sim.set_wire_drive(input_a, &LogicState::from_int(a))
//                .unwrap();
//            sim.set_wire_drive(input_b, &LogicState::from_int(b))
//                .unwrap();
//
//            match sim.run_sim(2) {
//                SimulationRunResult::Ok => {}
//                SimulationRunResult::MaxStepsReached => {
//                    panic!("[TEST ({a}, {b})] exceeded max steps")
//                }
//                SimulationRunResult::Err(err) => panic!("[TEST ({a}, {b})] {err:?}"),
//            }
//
//            let expected = LogicState::from_bool(compare_op(a, b));
//            let output_state = sim.get_wire_state(output).unwrap();
//
//            assert!(
//                output_state.eq(&expected, NonZeroU8::MIN),
//                "[TEST ({a}, {b})]  expected: {}  actual: {}",
//                expected.display_string(NonZeroU8::MIN),
//                output_state.display_string(NonZeroU8::MIN),
//            );
//        }
//    }
//}
//
//fn test_signed_comparator<F>(add_comparator: F, compare_op: impl Fn(i32, i32) -> bool)
//where
//    F: Fn(&mut SimulatorBuilder, WireId, WireId, WireId) -> AddComponentResult,
//{
//    const WIDTH: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(4) };
//
//    let mut builder = SimulatorBuilder::default();
//
//    let input_a = builder.add_wire(WIDTH).unwrap();
//    let input_b = builder.add_wire(WIDTH).unwrap();
//    let output = builder.add_wire(NonZeroU8::MIN).unwrap();
//    let _comparator = add_comparator(&mut builder, input_a, input_b, output).unwrap();
//
//    let mut sim = builder.build();
//
//    for a in -8..8 {
//        for b in -8..8 {
//            sim.set_wire_drive(input_a, &LogicState::from_int(a as u32))
//                .unwrap();
//            sim.set_wire_drive(input_b, &LogicState::from_int(b as u32))
//                .unwrap();
//
//            match sim.run_sim(2) {
//                SimulationRunResult::Ok => {}
//                SimulationRunResult::MaxStepsReached => {
//                    panic!("[TEST ({a}, {b})] exceeded max steps")
//                }
//                SimulationRunResult::Err(err) => panic!("[TEST ({a}, {b})] {err:?}"),
//            }
//
//            let expected = LogicState::from_bool(compare_op(a, b));
//            let output_state = sim.get_wire_state(output).unwrap();
//
//            assert!(
//                output_state.eq(&expected, NonZeroU8::MIN),
//                "[TEST ({a}, {b})]  expected: {}  actual: {}",
//                expected.display_string(NonZeroU8::MIN),
//                output_state.display_string(NonZeroU8::MIN),
//            );
//        }
//    }
//}
