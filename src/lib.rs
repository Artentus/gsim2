mod buffer;
mod gpu;
mod graph;
mod logic;
mod vec;

use buffer::*;
use bytemuck::{Pod, Zeroable};
use graph::*;
use logic::*;
use std::slice;

pub use logic::{
    FromBigIntError, FromBitsError, LogicBitState, LogicState, ParseError, ToIntError,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Zeroable, Pod)]
#[repr(transparent)]
pub struct WireId(Index<Wire>);

impl WireId {
    pub const INVALID: Self = Self(Index::INVALID);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Zeroable, Pod)]
#[repr(transparent)]
pub struct ComponentId(Index<Component>);

impl ComponentId {
    pub const INVALID: Self = Self(Index::INVALID);
}

pub const MIN_WIRE_WIDTH: u32 = 1;
pub const MAX_WIRE_WIDTH: u32 = (u8::MAX as u32) + 1;

#[derive(Debug, Clone)]
pub enum AddWireError {
    WidthOutOfRange,
    OutOfMemory,
}

impl From<BufferPushError> for AddWireError {
    fn from(err: BufferPushError) -> Self {
        match err {
            BufferPushError::OutOfMemory => AddWireError::OutOfMemory,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InvalidWireIdError;

#[derive(Debug, Clone)]
pub enum AddComponentError {
    InvalidWireId,
    TooManyInputs,
    OutOfMemory,
}

impl From<BufferPushError> for AddComponentError {
    fn from(err: BufferPushError) -> Self {
        match err {
            BufferPushError::OutOfMemory => AddComponentError::OutOfMemory,
        }
    }
}

macro_rules! gate_ports {
    ($ports:ident) => {
        #[derive(Debug, Clone)]
        pub struct $ports<'a> {
            pub inputs: &'a [WireId],
            pub output: WireId,
        }
    };
}

gate_ports!(AndGatePorts);
gate_ports!(OrGatePorts);
gate_ports!(XorGatePorts);
gate_ports!(NandGatePorts);
gate_ports!(NorGatePorts);
gate_ports!(XnorGatePorts);

macro_rules! horizontal_gate_ports {
    ($ports:ident) => {
        #[derive(Debug, Clone)]
        pub struct $ports {
            pub input: WireId,
            pub output: WireId,
        }
    };
}

horizontal_gate_ports!(HorizontalAndGatePorts);
horizontal_gate_ports!(HorizontalOrGatePorts);
horizontal_gate_ports!(HorizontalXorGatePorts);
horizontal_gate_ports!(HorizontalNandGatePorts);
horizontal_gate_ports!(HorizontalNorGatePorts);
horizontal_gate_ports!(HorizontalXnorGatePorts);

#[derive(Debug, Clone)]
pub struct NotGatePorts {
    pub input: WireId,
    pub output: WireId,
}

#[derive(Debug, Clone)]
pub struct BufferPorts {
    pub input: WireId,
    pub enable: WireId,
    pub output: WireId,
}

macro_rules! arithmetic_ports {
    ($ports:ident) => {
        #[derive(Debug, Clone)]
        pub struct $ports {
            pub input_lhs: WireId,
            pub input_rhs: WireId,
            pub output: WireId,
        }
    };
}

arithmetic_ports!(AddPorts);
arithmetic_ports!(SubtractPorts);
arithmetic_ports!(LeftShiftPorts);
arithmetic_ports!(LogicalRightShiftPorts);
arithmetic_ports!(ArithmeticRightShiftPorts);
arithmetic_ports!(CompareEqual);
arithmetic_ports!(CompareNotEqual);
arithmetic_ports!(CompareUnsignedLessThan);
arithmetic_ports!(CompareUnsignedGreaterThan);
arithmetic_ports!(CompareUnsignedLessThanOrEqual);
arithmetic_ports!(CompareUnsignedGreaterThanEqual);
arithmetic_ports!(CompareSignedLessThan);
arithmetic_ports!(CompareSignedGreaterThan);
arithmetic_ports!(CompareSignedLessThanOrEqual);
arithmetic_ports!(CompareSignedGreaterThanEqual);

#[derive(Debug, Clone)]
pub struct NegatePorts {
    pub input: WireId,
    pub output: WireId,
}

/// The result of running a simulation
#[derive(Debug, Clone)]
#[must_use]
pub enum SimulationRunResult {
    /// The simulation settled
    Ok,
    /// The simulation did not settle within the maximum allowed steps
    MaxStepsReached,
    /// The simulation produced an error
    Err {
        /// A list of wires that had more than one driver
        conflicting_wires: Box<[WireId]>,
    },
}

macro_rules! wire_drive_fns {
    () => {
        pub fn set_wire_drive(
            &mut self,
            wire: WireId,
            new_drive: &LogicState,
        ) -> Result<(), InvalidWireIdError> {
            let wire = self.wires.get(wire.0).ok_or(InvalidWireIdError)?;

            let state_width = wire.width.div_ceil(LogicStateAtom::BITS);
            let drive = self
                .wire_drives
                .get_mut(wire.drive_offset, state_width)
                .expect("invalid wire drive offset");
            drive.copy_from_slice(&new_drive.0[..drive.len()]);

            Ok(())
        }

        pub fn get_wire_drive(&mut self, wire: WireId) -> Result<LogicState, InvalidWireIdError> {
            let wire = self.wires.get(wire.0).ok_or(InvalidWireIdError)?;

            let state_width = wire.width.div_ceil(LogicStateAtom::BITS);
            let drive = self
                .wire_drives
                .get(wire.drive_offset, state_width)
                .expect("invalid wire drive offset");

            let mut result = LogicState::HIGH_Z;
            result.0[..drive.len()].copy_from_slice(drive);
            Ok(result)
        }
    };
}

#[derive(Debug, Default)]
pub struct SimulatorBuilder {
    wire_states: LogicStateBuffer<WireState, Building>,
    wire_drives: LogicStateBuffer<WireBaseDrive, Building>,
    wire_drivers: Buffer<WireDriver, Building>,
    wires: Buffer<Wire, Building>,

    output_states: LogicStateBuffer<OutputState, Building>,
    outputs: Buffer<ComponentOutput, Building>,
    inputs: Buffer<ComponentInput, Building>,
    memory: LogicStateBuffer<Memory, Building>,
    components: Buffer<Component, Building>,
}

impl SimulatorBuilder {
    pub fn add_wire(&mut self, width: u32) -> Result<WireId, AddWireError> {
        if (width < MIN_WIRE_WIDTH) || (width > MAX_WIRE_WIDTH) {
            return Err(AddWireError::WidthOutOfRange);
        }

        let state_width = width.div_ceil(LogicStateAtom::BITS);
        let state_offset = self.wire_states.push(state_width)?;
        let drive_offset = self.wire_drives.push(state_width)?;

        let wire = Wire {
            width,
            state_offset,
            drive_offset,
            first_driver_width: 0,
            first_driver_offset: Offset::INVALID,
            driver_list: Index::INVALID,
        };

        let wire_index = self.wires.push(wire)?;
        Ok(WireId(wire_index))
    }

    wire_drive_fns!();

    pub fn add_component<Ports: ComponentPorts>(
        &mut self,
        ports: Ports,
    ) -> Result<ComponentId, AddComponentError> {
        let output_kind = ports.create_outputs(
            &mut self.wire_drivers,
            &mut self.wires,
            &mut self.output_states,
            &mut self.outputs,
        )?;
        let (first_input, input_count) = ports.create_inputs(&self.wires, &mut self.inputs)?;
        let (memory_offset, memory_size) = ports.create_memory(&mut self.memory)?;

        let (output_count, output) = match output_kind {
            ComponentOutputKind::Single(output) => (1, ComponentInlineOutput { output }),
            ComponentOutputKind::List(first_output, count) => {
                assert!(count >= 2);
                let output = ComponentInlineOutput {
                    first_output: ComponentFirstOutput {
                        padding: 0,
                        first_output,
                    },
                };
                (count, output)
            }
        };

        let component = Component {
            kind: Ports::COMPONENT_KIND,
            output_count,
            input_count,
            output,
            first_input,
            memory_offset,
            memory_size,
        };

        let component_index = self.components.push(component)?;
        Ok(ComponentId(component_index))
    }

    #[inline]
    pub fn build(self) -> Result<Simulator, ()> {
        gpu::create_simulator(self)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
struct ListData {
    wires_changed: u32,
    components_changed: u32,
    conflict_list_len: u32,
    has_conflicts: u32,
}

const WORKGROUP_SIZE: u32 = 64;

pub struct Simulator {
    device: wgpu::Device,
    queue: wgpu::Queue,

    list_data_buffer: wgpu::Buffer,
    conflict_list_buffer: wgpu::Buffer,

    wire_states: LogicStateBuffer<WireState, Finalized>,
    wire_drives: LogicStateBuffer<WireBaseDrive, Finalized>,
    wire_drivers: Buffer<WireDriver, Finalized>,
    wires: Buffer<Wire, Finalized>,

    output_states: LogicStateBuffer<OutputState, Finalized>,
    outputs: Buffer<ComponentOutput, Finalized>,
    inputs: Buffer<ComponentInput, Finalized>,
    memory: LogicStateBuffer<Memory, Finalized>,
    components: Buffer<Component, Finalized>,

    bind_group: wgpu::BindGroup,
    _wire_shader: wgpu::ShaderModule,
    wire_pipeline: wgpu::ComputePipeline,
    _component_shader: wgpu::ShaderModule,
    component_pipeline: wgpu::ComputePipeline,
    _reset_shader: wgpu::ShaderModule,
    reset_pipeline: wgpu::ComputePipeline,

    staging_buffer: Option<wgpu::Buffer>,
    wire_states_need_sync: bool,
    memory_needs_sync: bool,
}

impl Simulator {
    fn sync_wire_states(&mut self) {
        if self.wire_states_need_sync {
            self.wire_states
                .sync(&self.device, &self.queue, &mut self.staging_buffer);
            self.wire_states_need_sync = false;
        }
    }

    fn sync_memory(&mut self) {
        if self.memory_needs_sync {
            self.memory
                .sync(&self.device, &self.queue, &mut self.staging_buffer);
            self.memory_needs_sync = false;
        }
    }

    wire_drive_fns!();

    pub fn get_wire_state(&mut self, wire: WireId) -> Result<LogicState, InvalidWireIdError> {
        self.sync_wire_states();

        let wire = self.wires.get(wire.0).ok_or(InvalidWireIdError)?;

        let state_width = wire.width.div_ceil(LogicStateAtom::BITS);
        let state = self
            .wire_states
            .get(wire.state_offset, state_width)
            .expect("invalid wire state offset");

        let mut result = LogicState::HIGH_Z;
        result.0[..state.len()].copy_from_slice(state);
        Ok(result)
    }

    fn read_list_data(&mut self) -> ListData {
        let mut list_data = ListData::zeroed();

        gpu::read_buffer::<ListData>(
            &self.list_data_buffer,
            bytemuck::cast_slice_mut(slice::from_mut(&mut list_data)),
            &self.device,
            &self.queue,
            &mut self.staging_buffer,
        );

        list_data
    }

    pub fn run(&mut self, mut max_steps: u64) -> SimulationRunResult {
        const RESET_WIRES_CHANGED: u32 = 0x1;
        const RESET_COMPONENTS_CHANGED: u32 = 0x2;

        self.wire_states.update(&self.queue);
        self.wire_drives.update(&self.queue);
        self.wire_drivers.update(&self.queue);
        self.wires.update(&self.queue);

        self.output_states.update(&self.queue);
        self.outputs.update(&self.queue);
        self.inputs.update(&self.queue);
        self.memory.update(&self.queue);
        self.components.update(&self.queue);

        self.wire_states_need_sync = true;
        self.memory_needs_sync = true;

        self.queue.write_buffer(
            &self.list_data_buffer,
            0,
            bytemuck::bytes_of(&ListData {
                wires_changed: self.wires.len(),
                components_changed: self.components.len(),
                conflict_list_len: 0,
                has_conflicts: 0,
            }),
        );

        while max_steps > 0 {
            let mut encoder = self.device.create_command_encoder(&Default::default());

            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_bind_group(0, &self.bind_group, &[]);

                for _ in 0..32 {
                    pass.set_pipeline(&self.reset_pipeline);
                    pass.set_push_constants(0, bytemuck::bytes_of(&RESET_WIRES_CHANGED));
                    pass.dispatch_workgroups(1, 1, 1);

                    pass.set_pipeline(&self.wire_pipeline);
                    pass.dispatch_workgroups(self.wires.len().div_ceil(WORKGROUP_SIZE), 1, 1);

                    pass.set_pipeline(&self.reset_pipeline);
                    pass.set_push_constants(0, bytemuck::bytes_of(&RESET_COMPONENTS_CHANGED));
                    pass.dispatch_workgroups(1, 1, 1);

                    pass.set_pipeline(&self.component_pipeline);
                    pass.dispatch_workgroups(self.components.len().div_ceil(WORKGROUP_SIZE), 1, 1);

                    max_steps -= 1;
                    if max_steps == 0 {
                        break;
                    }
                }
            }

            self.queue.submit(Some(encoder.finish()));

            let list_data = self.read_list_data();
            if list_data.has_conflicts != 0 {
                let mut conflicting_wires =
                    vec![WireId::INVALID; list_data.conflict_list_len as usize].into_boxed_slice();

                gpu::read_buffer(
                    &self.conflict_list_buffer,
                    &mut conflicting_wires,
                    &self.device,
                    &self.queue,
                    &mut self.staging_buffer,
                );

                return SimulationRunResult::Err { conflicting_wires };
            } else if (list_data.wires_changed == 0) && (list_data.components_changed == 0) {
                return SimulationRunResult::Ok;
            }
        }

        SimulationRunResult::MaxStepsReached
    }

    pub fn reset(&mut self) {
        self.wire_states.reset();
        self.output_states.reset();
        self.memory.reset();

        self.wire_states_need_sync = false;
        self.memory_needs_sync = false;
    }
}

#[test]
fn run() {
    let mut builder = SimulatorBuilder::default();
    let input_a = builder.add_wire(1).unwrap();
    let input_b = builder.add_wire(1).unwrap();
    let output = builder.add_wire(1).unwrap();
    let gate = builder
        .add_component(AndGatePorts {
            inputs: &[input_a, input_b],
            output,
        })
        .unwrap();

    let mut sim = builder.build().unwrap();
    sim.set_wire_drive(input_a, &false.into()).unwrap();
    sim.set_wire_drive(input_b, &true.into()).unwrap();
    let result = sim.run(3);
    assert!(matches!(result, SimulationRunResult::Ok), "{result:?}");

    let input_a_state = sim.get_wire_state(input_a).unwrap();
    let input_b_state = sim.get_wire_state(input_b).unwrap();
    let output_state = sim.get_wire_state(output).unwrap();

    assert!(input_a_state.eq(&false.into(), 1));
    assert!(input_b_state.eq(&true.into(), 1));
    assert!(output_state.eq(&false.into(), 1));
}
