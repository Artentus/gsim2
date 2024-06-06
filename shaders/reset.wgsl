@compute @workgroup_size(1, 1, 1) 
fn main() {
    atomicAnd(&list_data.changed, ~reset_changed);
}
