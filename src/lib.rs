mod buffer;
mod gpu;
mod logic;
mod vec;

use buffer::*;
use bytemuck::{Pod, Zeroable};
use logic::*;
use private::*;
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

trait LinkedListNode: Pod + 'static {
    fn next(&self) -> Index<Self>;
    fn set_next(&mut self, index: Index<Self>);

    #[inline]
    fn set_next_checked(&mut self, index: Index<Self>) {
        assert!(self.next().is_invalid());
        self.set_next(index);
    }
}

macro_rules! impl_linked_list_node {
    ($t:ty => $next:ident) => {
        impl LinkedListNode for $t {
            #[inline]
            fn next(&self) -> Index<Self> {
                self.$next
            }

            #[inline]
            fn set_next(&mut self, index: Index<Self>) {
                self.$next = index
            }
        }
    };
}

fn linked_list_push<T: LinkedListNode>(
    buffer: &mut Buffer<T, Building>,
    first_index: &mut Index<T>,
    index: Index<T>,
) {
    let mut prev_index = *first_index;
    if let Some(first_item) = buffer.get(prev_index) {
        let mut next_index = first_item.next();
        while let Some(item) = buffer.get(next_index) {
            prev_index = next_index;
            next_index = item.next();
        }

        buffer.get_mut(prev_index).unwrap().set_next_checked(index);
    } else {
        *first_index = index;
    }
}

#[inline]
fn linked_list_iter<'a, T: LinkedListNode, S: BufferState>(
    buffer: &'a Buffer<T, S>,
    first_index: Index<T>,
) -> impl Iterator<Item = &'a T> {
    struct Iter<'a, T: LinkedListNode, S: BufferState> {
        buffer: &'a Buffer<T, S>,
        current: Index<T>,
    }

    impl<'a, T: LinkedListNode, S: BufferState> Iterator for Iter<'a, T, S> {
        type Item = &'a T;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.buffer
                .get(self.current)
                .inspect(|item| self.current = item.next())
        }
    }

    Iter {
        buffer,
        current: first_index,
    }
}

pub const MIN_WIRE_WIDTH: u32 = 1;
pub const MAX_WIRE_WIDTH: u32 = 256;

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
    OutOfMemory,
}

impl From<BufferPushError> for AddComponentError {
    fn from(err: BufferPushError) -> Self {
        match err {
            BufferPushError::OutOfMemory => AddComponentError::OutOfMemory,
        }
    }
}

macro_rules! gate_component_args {
    ($args:ident) => {
        #[derive(Debug, Clone)]
        pub struct $args<'a> {
            pub inputs: &'a [WireId],
            pub output: WireId,
        }
    };
}

gate_component_args!(AndGateArgs);
gate_component_args!(OrGateArgs);
gate_component_args!(XorGateArgs);
gate_component_args!(NandGateArgs);
gate_component_args!(NorGateArgs);
gate_component_args!(XnorGateArgs);

#[derive(Debug, Clone)]
pub struct NotGateArgs {
    pub input: WireId,
    pub output: WireId,
}

#[derive(Debug, Clone)]
pub struct BufferArgs {
    pub input: WireId,
    pub enable: WireId,
    pub output: WireId,
}

mod private {
    use super::*;
    use pod_enum::pod_enum;

    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    #[repr(C)]
    pub struct WireDriver {
        pub width: u32,
        pub output_state_offset: Offset<OutputState>,
        pub next_driver: Index<WireDriver>,
    }

    impl_linked_list_node!(WireDriver => next_driver);

    pub enum WireState {}
    pub enum WireBaseDrive {}

    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    #[repr(C)]
    pub struct Wire {
        pub width: u32,
        pub state_offset: Offset<WireState>,
        pub drive_offset: Offset<WireBaseDrive>,
        pub first_driver_width: u32,
        pub first_driver_offset: Offset<OutputState>,
        pub driver_list: Index<WireDriver>,
    }

    impl Wire {
        pub fn add_driver(
            &mut self,
            buffer: &mut Buffer<WireDriver, Building>,
            width: u32,
            output_state_offset: Offset<OutputState>,
        ) -> Result<(), AddComponentError> {
            if self.first_driver_offset == Offset::INVALID {
                self.first_driver_width = width;
                self.first_driver_offset = output_state_offset;
            } else {
                let new_driver = buffer.push(WireDriver {
                    width,
                    output_state_offset,
                    next_driver: Index::INVALID,
                })?;

                linked_list_push(buffer, &mut self.driver_list, new_driver);
            }

            Ok(())
        }
    }

    pub enum OutputState {}

    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    #[repr(C)]
    pub struct ComponentOutput {
        pub width: u32,
        pub state_offset: Offset<OutputState>,
    }

    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    #[repr(C)]
    pub struct ComponentInput {
        pub width: u32,
        pub wire_state_offset: Offset<WireState>,
    }

    #[pod_enum]
    #[derive(Eq, PartialOrd, Ord)]
    #[repr(u32)]
    pub enum ComponentKind {
        And = 0,
        Or = 1,
        Xor = 2,
        Nand = 3,
        Nor = 4,
        Xnor = 5,
        Not = 6,
        Buffer = 7,
        Add = 8,
        Sub = 9,
        Neg = 10,
        Lsh = 11,
        LRsh = 12,
        ARsh = 13,
        HAnd = 14,
        HOr = 15,
        HXor = 16,
        HNand = 17,
        HNor = 18,
        HXnor = 19,
        CmpEq = 20,
        CmpNe = 21,
        CmpUlt = 22,
        CmpUgt = 23,
        CmpUle = 24,
        CmpUge = 25,
        CmpSlt = 26,
        CmpSgt = 27,
        CmpSle = 28,
        CmpSge = 29,
    }

    impl Default for ComponentKind {
        #[inline]
        fn default() -> Self {
            Self { inner: 0 }
        }
    }

    pub enum Memory {}

    #[derive(Debug, Clone, Copy, Zeroable, Pod)]
    #[repr(C)]
    pub struct Component {
        pub kind: ComponentKind,
        pub first_output: Index<ComponentOutput>,
        pub output_count: u32,
        pub first_input: Index<ComponentInput>,
        pub input_count: u32,
        pub memory_offset: Offset<Memory>,
        pub memory_size: u32,
    }

    pub trait AddComponentArgs {
        const COMPONENT_KIND: ComponentKind;

        fn create_outputs(
            &self,
            wire_drivers: &mut Buffer<WireDriver, Building>,
            wires: &mut Buffer<Wire, Building>,
            output_states: &mut LogicStateBuffer<OutputState, Building>,
            outputs: &mut Buffer<ComponentOutput, Building>,
        ) -> Result<(Index<ComponentOutput>, u32), AddComponentError>;

        fn create_inputs(
            &self,
            wires: &Buffer<Wire, Building>,
            inputs: &mut Buffer<ComponentInput, Building>,
        ) -> Result<(Index<ComponentInput>, u32), AddComponentError>;

        fn create_memory(
            &self,
            memory: &mut LogicStateBuffer<Memory, Building>,
        ) -> Result<(Offset<Memory>, u32), AddComponentError>;
    }

    macro_rules! single_output {
        () => {
            fn create_outputs(
                &self,
                wire_drivers: &mut Buffer<WireDriver, Building>,
                wires: &mut Buffer<Wire, Building>,
                output_states: &mut LogicStateBuffer<OutputState, Building>,
                outputs: &mut Buffer<ComponentOutput, Building>,
            ) -> Result<(Index<ComponentOutput>, u32), AddComponentError> {
                let output_wire = wires
                    .get_mut(self.output.0)
                    .ok_or(AddComponentError::InvalidWireId)?;

                let state_width = output_wire.width.div_ceil(LogicStateAtom::BITS);
                let state_offset = output_states.push(state_width)?;
                output_wire.add_driver(wire_drivers, state_width, state_offset)?;

                let output = ComponentOutput {
                    width: output_wire.width,
                    state_offset,
                };

                let output_index = outputs.push(output)?;
                Ok((output_index, 1))
            }
        };
    }

    macro_rules! no_memory {
        () => {
            #[inline]
            fn create_memory(
                &self,
                _memory: &mut LogicStateBuffer<Memory, Building>,
            ) -> Result<(Offset<Memory>, u32), AddComponentError> {
                Ok((Offset::INVALID, 0))
            }
        };
    }

    macro_rules! impl_gate_component_args {
        ($args:ident => $kind:ident) => {
            impl AddComponentArgs for $args<'_> {
                const COMPONENT_KIND: ComponentKind = ComponentKind::$kind;

                single_output!();

                fn create_inputs(
                    &self,
                    wires: &Buffer<Wire, Building>,
                    inputs: &mut Buffer<ComponentInput, Building>,
                ) -> Result<(Index<ComponentInput>, u32), AddComponentError> {
                    let mut first_input_index = Index::INVALID;

                    for input in self.inputs {
                        let input_wire =
                            wires.get(input.0).ok_or(AddComponentError::InvalidWireId)?;

                        let input = ComponentInput {
                            width: input_wire.width,
                            wire_state_offset: input_wire.state_offset,
                        };

                        let input_index = inputs.push(input)?;
                        if first_input_index == Index::INVALID {
                            first_input_index = input_index;
                        }
                    }

                    Ok((first_input_index, self.inputs.len() as u32))
                }

                no_memory!();
            }
        };
    }

    impl_gate_component_args!(AndGateArgs => And);
    impl_gate_component_args!(OrGateArgs => Or);
    impl_gate_component_args!(XorGateArgs => Xor);
    impl_gate_component_args!(NandGateArgs => Nand);
    impl_gate_component_args!(NorGateArgs => Nor);
    impl_gate_component_args!(XnorGateArgs => Xnor);

    impl AddComponentArgs for NotGateArgs {
        const COMPONENT_KIND: ComponentKind = ComponentKind::Not;

        single_output!();

        fn create_inputs(
            &self,
            wires: &Buffer<Wire, Building>,
            inputs: &mut Buffer<ComponentInput, Building>,
        ) -> Result<(Index<ComponentInput>, u32), AddComponentError> {
            let input_wire = wires
                .get(self.input.0)
                .ok_or(AddComponentError::InvalidWireId)?;

            let input = ComponentInput {
                width: input_wire.width,
                wire_state_offset: input_wire.state_offset,
            };

            let input_index = inputs.push(input)?;
            Ok((input_index, 1))
        }

        no_memory!();
    }

    impl AddComponentArgs for BufferArgs {
        const COMPONENT_KIND: ComponentKind = ComponentKind::Not;

        single_output!();

        fn create_inputs(
            &self,
            wires: &Buffer<Wire, Building>,
            inputs: &mut Buffer<ComponentInput, Building>,
        ) -> Result<(Index<ComponentInput>, u32), AddComponentError> {
            let input_wire = wires
                .get(self.input.0)
                .ok_or(AddComponentError::InvalidWireId)?;

            let input = ComponentInput {
                width: input_wire.width,
                wire_state_offset: input_wire.state_offset,
            };

            let input_index = inputs.push(input)?;

            let enable_wire = wires
                .get(self.enable.0)
                .ok_or(AddComponentError::InvalidWireId)?;

            let enable = ComponentInput {
                width: enable_wire.width,
                wire_state_offset: enable_wire.state_offset,
            };

            inputs.push(enable)?;

            Ok((input_index, 2))
        }

        no_memory!();
    }
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

    pub fn add_component<Args: AddComponentArgs>(
        &mut self,
        args: Args,
    ) -> Result<ComponentId, AddComponentError> {
        let (first_output, output_count) = args.create_outputs(
            &mut self.wire_drivers,
            &mut self.wires,
            &mut self.output_states,
            &mut self.outputs,
        )?;
        let (first_input, input_count) = args.create_inputs(&self.wires, &mut self.inputs)?;
        let (memory_offset, memory_size) = args.create_memory(&mut self.memory)?;

        let component = Component {
            kind: Args::COMPONENT_KIND,
            first_output,
            output_count,
            first_input,
            input_count,
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
    changed: u32,
    conflict_list_len: u32,
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
    output_states_need_sync: bool,
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

    fn sync_output_states(&mut self) {
        if self.output_states_need_sync {
            self.output_states
                .sync(&self.device, &self.queue, &mut self.staging_buffer);
            self.output_states_need_sync = false;
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
        const WIRE_STATES_CHANGED: u32 = 0x1;
        const COMPONENT_STATES_CHANGED: u32 = 0x2;

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
        self.output_states_need_sync = true;
        self.memory_needs_sync = true;

        self.queue.write_buffer(
            &self.list_data_buffer,
            0,
            bytemuck::bytes_of(&ListData {
                changed: WIRE_STATES_CHANGED | COMPONENT_STATES_CHANGED,
                conflict_list_len: 0,
            }),
        );

        while max_steps > 0 {
            let mut encoder = self.device.create_command_encoder(&Default::default());

            {
                let mut pass = encoder.begin_compute_pass(&Default::default());
                pass.set_bind_group(0, &self.bind_group, &[]);

                for _ in 0..32 {
                    pass.set_pipeline(&self.reset_pipeline);
                    pass.set_push_constants(0, bytemuck::bytes_of(&WIRE_STATES_CHANGED));
                    pass.dispatch_workgroups(1, 1, 1);

                    pass.set_pipeline(&self.wire_pipeline);
                    pass.dispatch_workgroups(self.wires.len().div_ceil(WORKGROUP_SIZE), 1, 1);

                    pass.set_pipeline(&self.reset_pipeline);
                    pass.set_push_constants(0, bytemuck::bytes_of(&COMPONENT_STATES_CHANGED));
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
            if list_data.conflict_list_len > 0 {
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
            } else if list_data.changed == 0 {
                return SimulationRunResult::Ok;
            }
        }

        SimulationRunResult::MaxStepsReached
    }

    pub fn reset(&mut self) {
        self.wire_states.reset();
        self.output_states.reset();
        self.memory.reset();
    }
}

#[test]
fn run() {
    let mut builder = SimulatorBuilder::default();
    let input_a = builder.add_wire(1).unwrap();
    let input_b = builder.add_wire(1).unwrap();
    let output = builder.add_wire(1).unwrap();
    let gate = builder
        .add_component(AndGateArgs {
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
