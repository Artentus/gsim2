struct LogicStateAtom {
    state: u32,
    valid: u32,
}

fn logic_state_equal(a: LogicStateAtom, b: LogicStateAtom) -> bool {
    return (a.state == b.state) && (a.valid == b.valid);
}

fn high_z_to_undefined(v: LogicStateAtom) -> LogicStateAtom {
    return LogicStateAtom(
        v.state | ~v.valid,
        v.valid,
    );
}

const HIGH_Z    = LogicStateAtom(0x00000000u, 0x00000000u);
const UNDEFINED = LogicStateAtom(0xFFFFFFFFu, 0x00000000u);
const LOGIC_0   = LogicStateAtom(0x00000000u, 0xFFFFFFFFu);
const LOGIC_1   = LogicStateAtom(0xFFFFFFFFu, 0xFFFFFFFFu);

struct LogicBitState {
    state: bool,
    valid: bool,
}

fn get_bit_state(v: LogicStateAtom, index: u32) -> LogicBitState {
    return LogicBitState(
        ((v.state >> index) & 0x1u) != 0u,
        ((v.valid >> index) & 0x1u) != 0u,
    );
}

fn logic_and(a: LogicStateAtom, b: LogicStateAtom) -> LogicStateAtom {
    //  A state | A valid | A meaning | B state | B valid | B meaning | O state | O valid | O meaning
    // ---------|---------|-----------|---------|---------|-----------|---------|---------|-----------
    //     0    |    0    | High-Z    |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    0    |    0    | High-Z    |    0    |    1    | Logic 0
    //     1    |    1    | Logic 1   |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     0    |    0    | High-Z    |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    1    |    0    | Undefined |    0    |    1    | Logic 0
    //     1    |    1    | Logic 1   |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     0    |    0    | High-Z    |    0    |    1    | Logic 0   |    0    |    1    | Logic 0
    //     1    |    0    | Undefined |    0    |    1    | Logic 0   |    0    |    1    | Logic 0
    //     0    |    1    | Logic 0   |    0    |    1    | Logic 0   |    0    |    1    | Logic 0
    //     1    |    1    | Logic 1   |    0    |    1    | Logic 0   |    0    |    1    | Logic 0
    //     0    |    0    | High-Z    |    1    |    1    | Logic 1   |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    1    |    1    | Logic 1   |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    1    |    1    | Logic 1   |    0    |    1    | Logic 0
    //     1    |    1    | Logic 1   |    1    |    1    | Logic 1   |    1    |    1    | Logic 1

    let state = ( a.state &  b.state)
              | (~a.valid & ~b.valid)
              | ( a.state & ~b.valid)
              | ( b.state & ~a.valid);

    let valid = ( a.valid & b.valid)
              | (~a.state & a.valid)
              | (~b.state & b.valid);

    return LogicStateAtom(state, valid);
}

fn logic_or(a: LogicStateAtom, b: LogicStateAtom) -> LogicStateAtom {
    //  A state | A valid | A meaning | B state | B valid | B meaning | O state | O valid | O meaning
    // ---------|---------|-----------|---------|---------|-----------|---------|---------|-----------
    //     0    |    0    | High-Z    |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     1    |    1    | Logic 1   |    0    |    0    | High-Z    |    1    |    1    | Logic 1
    //     0    |    0    | High-Z    |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     1    |    1    | Logic 1   |    1    |    0    | Undefined |    1    |    1    | Logic 1
    //     0    |    0    | High-Z    |    0    |    1    | Logic 0   |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    0    |    1    | Logic 0   |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    0    |    1    | Logic 0   |    0    |    1    | Logic 0
    //     1    |    1    | Logic 1   |    0    |    1    | Logic 0   |    1    |    1    | Logic 1
    //     0    |    0    | High-Z    |    1    |    1    | Logic 1   |    1    |    1    | Logic 1
    //     1    |    0    | Undefined |    1    |    1    | Logic 1   |    1    |    1    | Logic 1
    //     0    |    1    | Logic 0   |    1    |    1    | Logic 1   |    1    |    1    | Logic 1
    //     1    |    1    | Logic 1   |    1    |    1    | Logic 1   |    1    |    1    | Logic 1

    let state = a.state | ~a.valid | b.state | ~b.valid;

    let valid = (a.state & a.valid)
              | (b.state & b.valid)
              | (a.valid & b.valid);

    return LogicStateAtom(state, valid);
}

fn logic_xor(a: LogicStateAtom, b: LogicStateAtom) -> LogicStateAtom {
    //  A state | A valid | A meaning | B state | B valid | B meaning | O state | O valid | O meaning
    // ---------|---------|-----------|---------|---------|-----------|---------|---------|-----------
    //     0    |    0    | High-Z    |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     1    |    1    | Logic 1   |    0    |    0    | High-Z    |    1    |    0    | Undefined
    //     0    |    0    | High-Z    |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     1    |    1    | Logic 1   |    1    |    0    | Undefined |    1    |    0    | Undefined
    //     0    |    0    | High-Z    |    0    |    1    | Logic 0   |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    0    |    1    | Logic 0   |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    0    |    1    | Logic 0   |    0    |    1    | Logic 0
    //     1    |    1    | Logic 1   |    0    |    1    | Logic 0   |    1    |    1    | Logic 1
    //     0    |    0    | High-Z    |    1    |    1    | Logic 1   |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    1    |    1    | Logic 1   |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    1    |    1    | Logic 1   |    1    |    1    | Logic 1
    //     1    |    1    | Logic 1   |    1    |    1    | Logic 1   |    0    |    1    | Logic 0

    let state = (a.state ^ b.state) | ~a.valid | ~b.valid;
    let valid = a.valid & b.valid;

    return LogicStateAtom(state, valid);
}

fn logic_not(v: LogicStateAtom) -> LogicStateAtom {
    //  I state | I valid | I meaning | O state | O valid | O meaning
    // ---------|---------|-----------|---------|---------|-----------
    //     0    |    0    | High-Z    |    1    |    0    | Undefined
    //     1    |    0    | Undefined |    1    |    0    | Undefined
    //     0    |    1    | Logic 0   |    1    |    1    | Logic 1
    //     1    |    1    | Logic 1   |    0    |    1    | Logic 0

    return LogicStateAtom(~v.state | ~v.valid, v.valid);
}

struct WideningAddResult {
    sum: u32,
    carry: bool,
}

fn widening_add(a: u32, b: u32) -> WideningAddResult {
    let sum = a + b;
    return WideningAddResult(sum, sum < a);
}

fn carry_add(a: u32, b: u32, c: bool) -> WideningAddResult {
    let r1 = widening_add(a, b);
    let r2 = widening_add(r1.sum, u32(c));
    return WideningAddResult(r2.sum, r1.carry | r2.carry);
}

struct AddResult {
    sum: LogicStateAtom,
    carry: LogicBitState,
}

fn keep_trailing_ones(v: u32) -> u32 {
    let trailing_ones = countTrailingZeros(~v);

    if trailing_ones == 0u {
        return 0u;
    } else {
        return 0xFFFFFFFFu >> (32u - trailing_ones);
    }
}

fn logic_add(a: LogicStateAtom, b: LogicStateAtom, c: LogicBitState) -> AddResult {
    let r = carry_add(a.state, b.state, c.state);

    let mask_a = keep_trailing_ones(a.valid);
    let mask_b = keep_trailing_ones(b.valid);
    var valid = mask_a & mask_b;
    if !c.valid { valid = 0u; }
    let carry_valid = (valid >> 31u) > 0;
    
    return AddResult(
        LogicStateAtom(r.sum | ~valid, valid),
        LogicBitState(r.carry | !carry_valid, carry_valid),
    );
}

struct CombineResult {
    atom: LogicStateAtom,
    conflict: bool,
}

fn combine_state(a: LogicStateAtom, b: LogicStateAtom) -> CombineResult {
    //  A state | A valid | A meaning | B state | B valid | B meaning | O state | O valid | O meaning | conflict
    // ---------|---------|-----------|---------|---------|-----------|---------|---------|-----------|----------
    //     0    |    0    | High-Z    |    0    |    0    | High-Z    |    0    |    0    | High-Z    | no
    //     1    |    0    | Undefined |    0    |    0    | High-Z    |    1    |    0    | Undefined | no
    //     0    |    1    | Logic 0   |    0    |    0    | High-Z    |    0    |    1    | Logic 0   | no
    //     1    |    1    | Logic 1   |    0    |    0    | High-Z    |    1    |    1    | Logic 1   | no
    //     0    |    0    | High-Z    |    1    |    0    | Undefined |    1    |    0    | Undefined | no
    //     1    |    0    | Undefined |    1    |    0    | Undefined |    -    |    -    | -         | yes
    //     0    |    1    | Logic 0   |    1    |    0    | Undefined |    -    |    -    | -         | yes
    //     1    |    1    | Logic 1   |    1    |    0    | Undefined |    -    |    -    | -         | yes
    //     0    |    0    | High-Z    |    0    |    1    | Logic 0   |    0    |    1    | Logic 0   | no
    //     1    |    0    | Undefined |    0    |    1    | Logic 0   |    -    |    -    | -         | yes
    //     0    |    1    | Logic 0   |    0    |    1    | Logic 0   |    -    |    -    | -         | yes
    //     1    |    1    | Logic 1   |    0    |    1    | Logic 0   |    -    |    -    | -         | yes
    //     0    |    0    | High-Z    |    1    |    1    | Logic 1   |    1    |    1    | Logic 1   | no
    //     1    |    0    | Undefined |    1    |    1    | Logic 1   |    -    |    -    | -         | yes
    //     0    |    1    | Logic 0   |    1    |    1    | Logic 1   |    -    |    -    | -         | yes
    //     1    |    1    | Logic 1   |    1    |    1    | Logic 1   |    -    |    -    | -         | yes

    let state = a.state | b.state;
    let valid = a.valid | b.valid;

    let conflict = (a.state & b.state)
                 | (a.state & b.valid)
                 | (a.valid & b.state)
                 | (a.valid & b.valid);

    return CombineResult(LogicStateAtom(state, valid), conflict != 0u);
}

const MIN_WIRE_WIDTH = 1u;
const MAX_WIRE_WIDTH = 256u;

const ATOM_BITS = 32u;
const MAX_ATOM_COUNT = MAX_WIRE_WIDTH / ATOM_BITS;

const INVALID_INDEX = 0xFFFFFFFFu;

@group(0) @binding(0) 
var<storage, read_write> wire_states: array<LogicStateAtom>;

@group(0) @binding(1) 
var<storage, read> wire_drives: array<LogicStateAtom>;

struct WireDriver {
    width: u32,
    output_state_offset: u32,
    next_driver: u32,
}

@group(0) @binding(2) 
var<storage, read> wire_drivers: array<WireDriver>;

struct Wire {
    width: u32,
    state_offset: u32,
    drive_offset: u32,
    first_driver_width: u32,
    first_driver_offset: u32,
    driver_list: u32,
}

@group(0) @binding(3) 
var<storage, read> wires: array<Wire>;

@group(0) @binding(4) 
var<storage, read_write> output_states: array<LogicStateAtom>;

struct ComponentOutput {
    width: u32,
    state_offset: u32,
}

@group(0) @binding(5) 
var<storage, read> outputs: array<ComponentOutput>;

struct ComponentInput {
    width: u32,
    wire_state_offset: u32,
}

@group(0) @binding(6) 
var<storage, read> inputs: array<ComponentInput>;

@group(0) @binding(7) 
var<storage, read_write> memory: array<LogicStateAtom>;

const COMPONENT_KIND_AND    =  0u;
const COMPONENT_KIND_OR     =  1u;
const COMPONENT_KIND_XOR    =  2u;
const COMPONENT_KIND_NAND   =  3u;
const COMPONENT_KIND_NOR    =  4u;
const COMPONENT_KIND_XNOR   =  5u;
const COMPONENT_KIND_NOT    =  6u;
const COMPONENT_KIND_BUFFER =  7u;
const COMPONENT_KIND_ADD    =  8u;
const COMPONENT_KIND_SUB    =  9u;
const COMPONENT_KIND_NEG    = 10u;
const COMPONENT_KIND_LSH    = 11u;
const COMPONENT_KIND_LRSH   = 12u;
const COMPONENT_KIND_ARSH   = 13u;
const COMPONENT_KIND_HAND   = 14u;
const COMPONENT_KIND_HOR    = 15u;
const COMPONENT_KIND_HXOR   = 16u;
const COMPONENT_KIND_HNAND  = 17u;
const COMPONENT_KIND_HNOR   = 18u;
const COMPONENT_KIND_HXNOR  = 19u;
const COMPONENT_KIND_CMPEQ  = 20u;
const COMPONENT_KIND_CMPNE  = 21u;
const COMPONENT_KIND_CMPULT = 22u;
const COMPONENT_KIND_CMPUGT = 23u;
const COMPONENT_KIND_CMPULE = 24u;
const COMPONENT_KIND_CMPUGE = 25u;
const COMPONENT_KIND_CMPSLT = 26u;
const COMPONENT_KIND_CMPSGT = 27u;
const COMPONENT_KIND_CMPSLE = 28u;
const COMPONENT_KIND_CMPSGE = 29u;

struct PackedComponent {
    kind_output_count_input_count: u32,
    output_width: u32,
    output_offset_or_first_output: u32,
    first_input: u32,
    memory_offset: u32,
    memory_size: u32,
}

@group(0) @binding(8) 
var<storage, read> components: array<PackedComponent>;

struct ListData {
    wires_changed: atomic<u32>,
    components_changed: atomic<u32>,
    conflict_list_len: atomic<u32>,
    has_conflicts: atomic<u32>,
}

@group(0) @binding(9) 
var<storage, read_write> list_data: ListData;

@group(0) @binding(10) 
var<storage, read_write> conflict_list: array<u32>;

var<push_constant> reset_changed: u32;

struct Component {
    kind: u32,
    output_count: u32,
    input_count: u32,
    output_width: u32,
    output_offset_or_first_output: u32,
    first_input: u32,
    memory_offset: u32,
    memory_size: u32,
}

fn unpack_component(component: PackedComponent) -> Component {
    let kind = component.kind_output_count_input_count & 0xFFFFu;
    let output_count = (component.kind_output_count_input_count >> 16u) & 0xFFu;
    let input_count = (component.kind_output_count_input_count >> 24u) & 0xFFu;

    return Component(
        kind,
        output_count,
        input_count,
        component.output_width,
        component.output_offset_or_first_output,
        component.first_input,
        component.memory_offset,
        component.memory_size,
    );
}
