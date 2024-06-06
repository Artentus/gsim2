fn gate_impl(component: Component) -> bool {
    var new_state: array<LogicStateAtom, MAX_STATE_LEN>;
    let c_output = outputs[component.first_output];

    let first_input = inputs[component.first_input];
    for (var bit_index = 0u; bit_index < c_output.width; bit_index += 32u) {
        let index = bit_index / 32u;

        if bit_index < first_input.width {
            new_state[index] = wire_states[first_input.wire_state_offset + index];
        } else {
            new_state[index] = HIGH_Z;
        }
    }

    for (var input_index = 0u; input_index < component.input_count; input_index++) {
        let c_input = inputs[component.first_input + input_index];

        for (var bit_index = 0u; bit_index < c_output.width; bit_index += 32u) {
            let index = bit_index / 32u;

            var input_atom: LogicStateAtom;
            if bit_index < c_input.width {
                input_atom = wire_states[c_input.wire_state_offset + index];
            } else {
                input_atom = HIGH_Z;
            }

            switch component.kind {
                case COMPONENT_KIND_AND: {
                    new_state[index] = logic_and(new_state[index], input_atom);
                }
                case COMPONENT_KIND_OR: {
                    new_state[index] = logic_or(new_state[index], input_atom);
                }
                case COMPONENT_KIND_XOR: {
                    new_state[index] = logic_xor(new_state[index], input_atom);
                }
                case COMPONENT_KIND_NAND: {
                    new_state[index] = logic_nand(new_state[index], input_atom);
                }
                case COMPONENT_KIND_NOR: {
                    new_state[index] = logic_nor(new_state[index], input_atom);
                }
                case COMPONENT_KIND_XNOR: {
                    new_state[index] = logic_xnor(new_state[index], input_atom);
                }
                default: {}
            }
        }
    }

    var state_changed = false;
    for (var bit_index = 0u; bit_index < c_output.width; bit_index += 32u) {
        let index = bit_index / 32u;

        if !logic_state_equal(output_states[c_output.state_offset + index], new_state[index]) {
            output_states[c_output.state_offset + index] = new_state[index];
            state_changed = true;
        }
    }

    return state_changed;
}

fn not_impl(component: Component) -> bool {
    let c_output = outputs[component.first_output];
    let c_input = inputs[component.first_input];

    var state_changed = false;
    for (var bit_index = 0u; bit_index < c_output.width; bit_index += 32u) {
        let index = bit_index / 32u;

        var atom: LogicStateAtom;
        if bit_index < c_input.width {
            atom = wire_states[c_input.wire_state_offset + index];
        } else {
            atom = HIGH_Z;
        }
        atom = logic_not(atom);

        if !logic_state_equal(output_states[c_output.state_offset + index], atom) {
            output_states[c_output.state_offset + index] = atom;
            state_changed = true;
        }
    }

    return state_changed;
}

fn buffer_impl(component: Component) -> bool {
    let c_output = outputs[component.first_output];
    let c_input = inputs[component.first_input];
    let c_enable = inputs[component.first_output + 1u];

    let enable_state = (wire_states[c_enable.wire_state_offset].state & 0x1u) > 0u;
    let enable_valid = (wire_states[c_enable.wire_state_offset].valid & 0x1u) > 0u;

    var state_changed = false;
    for (var bit_index = 0u; bit_index < c_output.width; bit_index += 32u) {
        let index = bit_index / 32u;

        var atom: LogicStateAtom;
        if !enable_valid {
            atom = UNDEFINED;
        } else if (enable_state) && (bit_index < c_input.width) {
            atom = wire_states[c_input.wire_state_offset + index];
        } else {
            atom = HIGH_Z;
        }

        if !logic_state_equal(output_states[c_output.state_offset + index], atom) {
            output_states[c_output.state_offset + index] = atom;
            state_changed = true;
        }
    }

    return state_changed;
}

@compute @workgroup_size(64, 1, 1) 
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let wires_changed = (atomicLoad(&list_data.changed) & WIRE_STATES_CHANGED) != 0u;
    let has_conflicts = atomicLoad(&list_data.conflict_list_len) > 0u;
    if !wires_changed || has_conflicts {
        return;
    }

    let component_index = id.x;
    if component_index >= arrayLength(&components) {
        return;
    }
    let component = components[component_index];

    var state_changed = false;
    switch component.kind {
        case COMPONENT_KIND_AND,
          COMPONENT_KIND_OR,
          COMPONENT_KIND_XOR,
          COMPONENT_KIND_NAND,
          COMPONENT_KIND_NOR,
          COMPONENT_KIND_XNOR: {
            state_changed = gate_impl(component);
        }
        case COMPONENT_KIND_NOT: {
            state_changed = not_impl(component);
        }
        case COMPONENT_KIND_BUFFER: {
            state_changed = buffer_impl(component);
        }
        default: {}
    }

    if state_changed {
        atomicOr(&list_data.changed, COMPONENT_STATES_CHANGED);
    }
}
