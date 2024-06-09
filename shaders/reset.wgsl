const RESET_WIRES_CHANGED      = 0x1u;
const RESET_COMPONENTS_CHANGED = 0x2u;

@compute @workgroup_size(1, 1, 1) 
fn main() {
    if (reset_changed & RESET_WIRES_CHANGED) != 0u {
        atomicStore(&list_data.wires_changed, 0u);
    }
    
    if (reset_changed & RESET_COMPONENTS_CHANGED) != 0u {
        atomicStore(&list_data.components_changed, 0u);
    }

    let conflict_list_len = atomicLoad(&list_data.conflict_list_len);
    let has_conflicts = u32(conflict_list_len > 0u);
    atomicStore(&list_data.has_conflicts, has_conflicts);
}
