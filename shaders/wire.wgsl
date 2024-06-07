fn mark_conflict(wire_index: u32) {
    let list_index: u32 = atomicAdd(&list_data.conflict_list_len, 1u);
    if list_index < arrayLength(&conflict_list) {
        conflict_list[list_index] = wire_index;
    }
}

@compute @workgroup_size(64, 1, 1) 
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let components_changed = atomicLoad(&list_data.components_changed);
    let has_conflicts = atomicLoad(&list_data.has_conflicts);
    if (components_changed == 0u) || (has_conflicts != 0u) {
        return;
    }

    let wire_index = id.x;
    if wire_index >= arrayLength(&wires) {
        return;
    }
    let wire = wires[wire_index];

    var new_state: array<LogicStateAtom, MAX_STATE_LEN>;
    for (var bit_index = 0u; bit_index < wire.width; bit_index += 32u) {
        let index = bit_index / 32u;
        new_state[index] = wire_drives[wire.drive_offset + index];
    }

    var has_conflict = false;
    if wire.first_driver_offset != INVALID_INDEX {
        for (var bit_index = 0u; bit_index < min(wire.width, wire.first_driver_width); bit_index += 32u) {
            let index = bit_index / 32u;

            let output_state = output_states[wire.first_driver_offset + index];
            let combine_result = combine_state(new_state[index], output_state);
            has_conflict |= combine_result.conflict;
            new_state[index] = combine_result.atom;
        }

        var next_driver = wire.driver_list;
        while next_driver != INVALID_INDEX {
            let driver = wire_drivers[next_driver];
            next_driver = driver.next_driver;

            for (var bit_index = 0u; bit_index < min(wire.width, driver.width); bit_index += 32u) {
                let index = bit_index / 32u;

                let output_state = output_states[driver.output_state_offset + index];
                let combine_result = combine_state(new_state[index], output_state);
                has_conflict |= combine_result.conflict;
                new_state[index] = combine_result.atom;
            }
        }
    }

    var state_changed = false;
    for (var bit_index = 0u; bit_index < wire.width; bit_index += 32u) {
        let index = bit_index / 32u;

        let dst = &wire_states[wire.state_offset + index];
        let src = new_state[index];
        if !logic_state_equal(*dst, src) {
            *dst = src;
            state_changed = true;
        }
    }

    if state_changed {
        atomicAdd(&list_data.wires_changed, 1u);
    }

    if has_conflict {
        mark_conflict(wire_index);
    }
}
