use criterion::{criterion_group, criterion_main, Criterion};
use gsim2::*;

fn generate_sim() -> Simulator {
    use rand::distributions::Uniform;
    use rand::prelude::*;

    let mut rng = StdRng::seed_from_u64(0);
    let drive_dist = Uniform::new(0, 3);
    let comp_dist = Uniform::new(0, 8);

    let mut builder = SimulatorBuilder::default();
    let mut wires = Vec::new();

    for _ in 0..100 {
        let wire = builder.add_wire(1).unwrap();
        let drive = match drive_dist.sample(&mut rng) {
            0 => LogicState::HIGH_Z,
            1 => LogicState::LOGIC_0,
            2 => LogicState::LOGIC_1,
            _ => unreachable!(),
        };
        builder.set_wire_drive(wire, &drive).unwrap();
        wires.push(wire);
    }

    let mut and_gates = Vec::new();
    let mut or_gates = Vec::new();
    let mut xor_gates = Vec::new();
    let mut nand_gates = Vec::new();
    let mut nor_gates = Vec::new();
    let mut xnor_gates = Vec::new();
    let mut not_gates = Vec::new();
    let mut buffers = Vec::new();

    for _ in 0..1000000 {
        let output = builder.add_wire(1).unwrap();
        match comp_dist.sample(&mut rng) {
            0 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                and_gates.push((input_a, input_b, output));
            }
            1 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                or_gates.push((input_a, input_b, output));
            }
            2 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                xor_gates.push((input_a, input_b, output));
            }
            3 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                nand_gates.push((input_a, input_b, output));
            }
            4 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                nor_gates.push((input_a, input_b, output));
            }
            5 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                xnor_gates.push((input_a, input_b, output));
            }
            6 => {
                let input = *wires.choose(&mut rng).unwrap();
                not_gates.push((input, output));
            }
            7 => {
                let input = *wires.choose(&mut rng).unwrap();
                let enable = *wires.choose(&mut rng).unwrap();
                buffers.push((input, enable, output));
            }
            _ => unreachable!(),
        }
        wires.push(output);
    }

    for (input_a, input_b, output) in and_gates {
        let _id = builder
            .add_component(AndGateArgs {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in or_gates {
        let _id = builder
            .add_component(OrGateArgs {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in xor_gates {
        let _id = builder
            .add_component(XorGateArgs {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in nand_gates {
        let _id = builder
            .add_component(NandGateArgs {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in nor_gates {
        let _id = builder
            .add_component(NorGateArgs {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in xnor_gates {
        let _id = builder
            .add_component(XnorGateArgs {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input, output) in not_gates {
        let _id = builder
            .add_component(NotGateArgs { input, output })
            .unwrap();
    }

    for (input, enable, output) in buffers {
        let _id = builder
            .add_component(BufferArgs {
                input,
                enable,
                output,
            })
            .unwrap();
    }

    let sim = builder.build().unwrap();

    //if first {
    //    let stats = sim.stats();

    //    println!();
    //    println!();
    //    println!("Wires: {} ({})", stats.wire_count, stats.wire_alloc_size);
    //    println!("    Width alloc: {}", stats.wire_width_alloc_size);
    //    println!("    Drive alloc: {}", stats.wire_drive_alloc_size);
    //    println!("    State alloc: {}", stats.wire_state_alloc_size);
    //    println!(
    //        "Components: {} + {} ({} + {})",
    //        stats.small_component_count,
    //        stats.large_component_count,
    //        stats.component_alloc_size,
    //        stats.large_component_alloc_size
    //    );
    //    println!("    Width alloc: {}", stats.output_width_alloc_size);
    //    println!("    State alloc: {}", stats.output_state_alloc_size);
    //    println!(
    //        "Total memory: {}",
    //        stats.wire_alloc_size
    //            + stats.wire_width_alloc_size
    //            + stats.wire_drive_alloc_size
    //            + stats.wire_state_alloc_size
    //            + stats.component_alloc_size
    //            + stats.large_component_alloc_size
    //            + stats.output_width_alloc_size
    //            + stats.output_state_alloc_size
    //    );
    //}

    sim
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut sim = generate_sim();

    c.bench_function("sim", |b| {
        b.iter(|| {
            sim.reset();
            let result = sim.run(u64::MAX);
            assert!(matches!(result, SimulationRunResult::Ok));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
