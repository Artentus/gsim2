use crate::*;
use pod_enum::pod_enum;
use std::fmt;

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

#[pod_enum]
#[derive(Eq, PartialOrd, Ord)]
#[repr(u16)]
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

pub enum OutputState {}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct ComponentOutput {
    pub width: u32,
    pub state_offset: Offset<OutputState>,
}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct ComponentFirstOutput {
    pub padding: u32,
    pub first_output: Index<ComponentOutput>,
}

#[derive(Clone, Copy, Zeroable)]
#[repr(C)]
pub union ComponentInlineOutput {
    pub output: ComponentOutput,
    pub first_output: ComponentFirstOutput,
}

unsafe impl Pod for ComponentInlineOutput {}

#[derive(Debug, Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct ComponentInput {
    pub width: u32,
    pub wire_state_offset: Offset<WireState>,
}

pub enum Memory {}

#[derive(Clone, Copy, Zeroable, Pod)]
#[repr(C)]
pub struct Component {
    pub kind: ComponentKind,
    pub output_count: u8,
    pub input_count: u8,
    pub output: ComponentInlineOutput,
    pub first_input: Index<ComponentInput>,
    pub memory_offset: Offset<Memory>,
    pub memory_size: u32,
}

impl fmt::Debug for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_struct = f.debug_struct("Component");
        debug_struct
            .field("kind", &self.kind)
            .field("output_count", &self.output_count)
            .field("input_count", &self.input_count);

        if self.output_count == 1 {
            debug_struct.field("output", unsafe { &self.output.output });
        } else {
            debug_struct.field("output", unsafe { &self.output.first_output });
        }

        debug_struct
            .field("first_input", &self.first_input)
            .field("memory_offset", &self.memory_offset)
            .field("memory_size", &self.memory_size)
            .finish()
    }
}

pub enum ComponentOutputKind {
    Single(ComponentOutput),
    List(Index<ComponentOutput>, u8),
}

pub trait ComponentPorts {
    const COMPONENT_KIND: ComponentKind;

    fn create_outputs(
        &self,
        wire_drivers: &mut Buffer<WireDriver, Building>,
        wires: &mut Buffer<Wire, Building>,
        output_states: &mut LogicStateBuffer<OutputState, Building>,
        outputs: &mut Buffer<ComponentOutput, Building>,
    ) -> Result<ComponentOutputKind, AddComponentError>;

    fn create_inputs(
        &self,
        wires: &Buffer<Wire, Building>,
        inputs: &mut Buffer<ComponentInput, Building>,
    ) -> Result<(Index<ComponentInput>, u8), AddComponentError>;

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
            _outputs: &mut Buffer<ComponentOutput, Building>,
        ) -> Result<ComponentOutputKind, AddComponentError> {
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

            Ok(ComponentOutputKind::Single(output))
        }
    };
}

macro_rules! single_input {
    () => {
        fn create_inputs(
            &self,
            wires: &Buffer<Wire, Building>,
            inputs: &mut Buffer<ComponentInput, Building>,
        ) -> Result<(Index<ComponentInput>, u8), AddComponentError> {
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

macro_rules! impl_gate_ports {
    ($args:ident => $kind:ident) => {
        impl ComponentPorts for $args<'_> {
            const COMPONENT_KIND: ComponentKind = ComponentKind::$kind;

            single_output!();

            fn create_inputs(
                &self,
                wires: &Buffer<Wire, Building>,
                inputs: &mut Buffer<ComponentInput, Building>,
            ) -> Result<(Index<ComponentInput>, u8), AddComponentError> {
                let input_count: u8 = self
                    .inputs
                    .len()
                    .try_into()
                    .map_err(|_| AddComponentError::TooManyInputs)?;

                let mut first_input_index = Index::INVALID;
                for input in self.inputs {
                    let input_wire = wires.get(input.0).ok_or(AddComponentError::InvalidWireId)?;

                    let input = ComponentInput {
                        width: input_wire.width,
                        wire_state_offset: input_wire.state_offset,
                    };

                    let input_index = inputs.push(input)?;
                    if first_input_index == Index::INVALID {
                        first_input_index = input_index;
                    }
                }

                Ok((first_input_index, input_count))
            }

            no_memory!();
        }
    };
}

impl_gate_ports!(AndGatePorts => And);
impl_gate_ports!(OrGatePorts => Or);
impl_gate_ports!(XorGatePorts => Xor);
impl_gate_ports!(NandGatePorts => Nand);
impl_gate_ports!(NorGatePorts => Nor);
impl_gate_ports!(XnorGatePorts => Xnor);

macro_rules! impl_horizontal_gate_ports {
    ($args:ident => $kind:ident) => {
        impl ComponentPorts for $args {
            const COMPONENT_KIND: ComponentKind = ComponentKind::$kind;

            single_output!();
            single_input!();
            no_memory!();
        }
    };
}

impl_horizontal_gate_ports!(HorizontalAndGatePorts => HAnd);
impl_horizontal_gate_ports!(HorizontalOrGatePorts => HOr);
impl_horizontal_gate_ports!(HorizontalXorGatePorts => HXor);
impl_horizontal_gate_ports!(HorizontalNandGatePorts => HNand);
impl_horizontal_gate_ports!(HorizontalNorGatePorts => HNor);
impl_horizontal_gate_ports!(HorizontalXnorGatePorts => HXnor);

impl ComponentPorts for NotGatePorts {
    const COMPONENT_KIND: ComponentKind = ComponentKind::Not;

    single_output!();
    single_input!();
    no_memory!();
}

impl ComponentPorts for BufferPorts {
    const COMPONENT_KIND: ComponentKind = ComponentKind::Not;

    single_output!();

    fn create_inputs(
        &self,
        wires: &Buffer<Wire, Building>,
        inputs: &mut Buffer<ComponentInput, Building>,
    ) -> Result<(Index<ComponentInput>, u8), AddComponentError> {
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

macro_rules! impl_arithmetic_ports {
    ($args:ident => $kind:ident) => {
        impl ComponentPorts for $args {
            const COMPONENT_KIND: ComponentKind = ComponentKind::$kind;

            single_output!();

            fn create_inputs(
                &self,
                wires: &Buffer<Wire, Building>,
                inputs: &mut Buffer<ComponentInput, Building>,
            ) -> Result<(Index<ComponentInput>, u8), AddComponentError> {
                let input_lhs_wire = wires
                    .get(self.input_lhs.0)
                    .ok_or(AddComponentError::InvalidWireId)?;

                let input_lhs = ComponentInput {
                    width: input_lhs_wire.width,
                    wire_state_offset: input_lhs_wire.state_offset,
                };

                let input_lhs_index = inputs.push(input_lhs)?;

                let input_rhs_wire = wires
                    .get(self.input_rhs.0)
                    .ok_or(AddComponentError::InvalidWireId)?;

                let input_rhs = ComponentInput {
                    width: input_rhs_wire.width,
                    wire_state_offset: input_rhs_wire.state_offset,
                };

                inputs.push(input_rhs)?;

                Ok((input_lhs_index, 2))
            }

            no_memory!();
        }
    };
}

impl_arithmetic_ports!(AddPorts => Add);
impl_arithmetic_ports!(SubtractPorts => Sub);
impl_arithmetic_ports!(LeftShiftPorts => Lsh);
impl_arithmetic_ports!(LogicalRightShiftPorts => LRsh);
impl_arithmetic_ports!(ArithmeticRightShiftPorts => ARsh);

impl ComponentPorts for NegatePorts {
    const COMPONENT_KIND: ComponentKind = ComponentKind::Neg;

    single_output!();
    single_input!();
    no_memory!();
}
