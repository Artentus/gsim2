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

    for _ in 0..1000000 {
        let output = builder.add_wire(1).unwrap();
        match comp_dist.sample(&mut rng) {
            0 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(AndGatePorts {
                        inputs: &[input_a, input_b],
                        output,
                    })
                    .unwrap();
            }
            1 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(OrGatePorts {
                        inputs: &[input_a, input_b],
                        output,
                    })
                    .unwrap();
            }
            2 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(XorGatePorts {
                        inputs: &[input_a, input_b],
                        output,
                    })
                    .unwrap();
            }
            3 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(NandGatePorts {
                        inputs: &[input_a, input_b],
                        output,
                    })
                    .unwrap();
            }
            4 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(NorGatePorts {
                        inputs: &[input_a, input_b],
                        output,
                    })
                    .unwrap();
            }
            5 => {
                let input_a = *wires.choose(&mut rng).unwrap();
                let input_b = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(XnorGatePorts {
                        inputs: &[input_a, input_b],
                        output,
                    })
                    .unwrap();
            }
            6 => {
                let input = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(NotGatePorts { input, output })
                    .unwrap();
            }
            7 => {
                let input = *wires.choose(&mut rng).unwrap();
                let enable = *wires.choose(&mut rng).unwrap();
                let _id = builder
                    .add_component(BufferPorts {
                        input,
                        enable,
                        output,
                    })
                    .unwrap();
            }
            _ => unreachable!(),
        }
        wires.push(output);
    }

    builder.build().unwrap()
}

fn generate_sim_sorted() -> Simulator {
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
            .add_component(AndGatePorts {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in or_gates {
        let _id = builder
            .add_component(OrGatePorts {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in xor_gates {
        let _id = builder
            .add_component(XorGatePorts {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in nand_gates {
        let _id = builder
            .add_component(NandGatePorts {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in nor_gates {
        let _id = builder
            .add_component(NorGatePorts {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input_a, input_b, output) in xnor_gates {
        let _id = builder
            .add_component(XnorGatePorts {
                inputs: &[input_a, input_b],
                output,
            })
            .unwrap();
    }

    for (input, output) in not_gates {
        let _id = builder
            .add_component(NotGatePorts { input, output })
            .unwrap();
    }

    for (input, enable, output) in buffers {
        let _id = builder
            .add_component(BufferPorts {
                input,
                enable,
                output,
            })
            .unwrap();
    }

    builder.build().unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut sim = generate_sim();
    let mut sorted_sim = generate_sim_sorted();

    c.benchmark_group("random graph")
        .bench_function("random insertion order", |b| {
            b.iter(|| {
                sim.reset();
                let result = sim.run(u64::MAX);
                assert!(matches!(result, SimulationRunResult::Ok));
            })
        })
        .bench_function("sorted insertion order", |b| {
            b.iter(|| {
                sorted_sim.reset();
                let result = sorted_sim.run(u64::MAX);
                assert!(matches!(result, SimulationRunResult::Ok));
            })
        });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
