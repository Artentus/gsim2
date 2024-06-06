fn mark_conflict(wire_index: u32) {
    let list_index: u32 = atomicAdd(&list_data.conflict_list_len, 1u);
    if list_index < arrayLength(&conflict_list) {
        conflict_list[list_index] = wire_index;
    }
}

@compute @workgroup_size(64, 1, 1) 
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let components_changed = (atomicLoad(&list_data.changed) & COMPONENT_STATES_CHANGED) != 0u;
    let has_conflicts = atomicLoad(&list_data.conflict_list_len) > 0u;
    if !components_changed || has_conflicts {
        return;
    }

    let wire_index = id.x;
    if wire_index >= arrayLength(&wires) {
        return;
    }
    let wire = wires[wire_index];

    var new_state: array<WireStateAtom, MAX_STATE_LEN>;
    for (var bit_index = 0u; bit_index < wire.width; bit_index += 32u) {
        let index = bit_index / 32u;
        new_state[index] = WireStateAtom(wire_drives[wire.drive_offset + index], 0u);
    }

    if wire.first_driver_offset != INVALID_INDEX {
        for (var bit_index = 0u; bit_index < min(wire.width, wire.first_driver_width); bit_index += 32u) {
            let index = bit_index / 32u;

            let output_state = output_states[wire.first_driver_offset + index];
            new_state[index] = combine_state(new_state[index], output_state);
        }

        var next_driver = wire.driver_list;
        while next_driver != INVALID_INDEX {
            let driver = wire_drivers[next_driver];
            next_driver = driver.next_driver;

            for (var bit_index = 0u; bit_index < min(wire.width, driver.width); bit_index += 32u) {
                let index = bit_index / 32u;

                let output_state = output_states[driver.output_state_offset + index];
                new_state[index] = combine_state(new_state[index], output_state);
            }
        }
    }

    var state_changed = false;
    var has_conflict = false;
    for (var bit_index = 0u; bit_index < wire.width; bit_index += 32u) {
        let index = bit_index / 32u;

        if !logic_state_equal(wire_states[wire.state_offset + index], new_state[index].state) {
            wire_states[wire.state_offset + index] = new_state[index].state;
            state_changed = true;
        }

        if new_state[index].conflict != 0u {
            has_conflict = true;
        }
    }

    if state_changed {
        atomicOr(&list_data.changed, WIRE_STATES_CHANGED);
    }

    if has_conflict {
        mark_conflict(wire_index);
    }
}
