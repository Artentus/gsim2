use crate::*;
use bytemuck::Pod;
use std::mem;
use std::slice;
use std::sync::OnceLock;
use wgpu::Buffer;
use wgpu::*;

fn create_device() -> (Device, Queue) {
    let instance_desc = InstanceDescriptor {
        backends: Backends::VULKAN | Backends::METAL,
        ..Default::default()
    };
    let instance = Instance::new(instance_desc);

    let adapter_opts = RequestAdapterOptions {
        power_preference: PowerPreference::HighPerformance,
        ..Default::default()
    };
    let adapter = pollster::block_on(instance.request_adapter(&adapter_opts))
        .expect("graphics adapter not found");

    let adapter_limits = adapter.limits();
    let device_limits = Limits {
        max_bind_groups: 2,
        max_bindings_per_bind_group: 16,
        max_storage_buffers_per_shader_stage: 16,
        max_push_constant_size: 128,

        max_storage_buffer_binding_size: adapter_limits.max_storage_buffer_binding_size,
        max_compute_invocations_per_workgroup: adapter_limits.max_compute_invocations_per_workgroup,
        max_compute_workgroup_size_x: adapter_limits.max_compute_workgroup_size_x,
        max_compute_workgroup_size_y: adapter_limits.max_compute_workgroup_size_y,
        max_compute_workgroup_size_z: adapter_limits.max_compute_workgroup_size_z,
        max_compute_workgroups_per_dimension: adapter_limits.max_compute_workgroups_per_dimension,
        min_subgroup_size: adapter_limits.min_subgroup_size,
        max_subgroup_size: adapter_limits.max_subgroup_size,
        ..Limits::downlevel_defaults()
    };

    let device_desc = DeviceDescriptor {
        required_limits: device_limits,
        required_features: Features::PUSH_CONSTANTS,
        ..Default::default()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&device_desc, None))
        .expect("graphics device not supported");

    (device, queue)
}

fn device() -> &'static (Device, Queue) {
    static DEVICE: OnceLock<(Device, Queue)> = OnceLock::new();
    DEVICE.get_or_init(create_device)
}

pub fn read_buffer<T: Pod>(
    buffer: &Buffer,
    dst: &mut [T],
    device: &Device,
    queue: &Queue,
    staging_buffer: &mut Option<Buffer>,
) {
    assert!(buffer.size() >= (dst.len() * mem::size_of::<T>()) as u64);

    if !staging_buffer
        .as_ref()
        .is_some_and(|staging_buffer| staging_buffer.size() >= buffer.size())
    {
        *staging_buffer = Some(device.create_buffer(&BufferDescriptor {
            label: None,
            size: buffer.size() * 2,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        }));
    }

    let staging_buffer = staging_buffer.as_ref().unwrap();

    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, buffer.size());
    queue.submit(Some(encoder.finish()));

    let staging_slice = staging_buffer.slice(..buffer.size());
    staging_slice.map_async(MapMode::Read, |result| result.unwrap());
    device.poll(Maintain::wait()).panic_on_timeout();

    let staging_view = staging_slice.get_mapped_range();
    let dst: &mut [u8] = bytemuck::cast_slice_mut(dst);
    let src: &[u8] = &staging_view[..dst.len()];
    dst.copy_from_slice(src);

    mem::drop(staging_view);
    staging_buffer.unmap();
}

const BIND_GROUP_ENTRIES: &[BindGroupLayoutEntry] = &[
    BindGroupLayoutEntry {
        binding: 0,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<LogicStateAtom>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 1,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<LogicStateAtom>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 2,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<WireDriver>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 3,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<Wire>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 4,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<LogicStateAtom>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 5,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<ComponentOutput>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 6,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<ComponentInput>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 7,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<LogicStateAtom>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 8,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<Component>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 9,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<ListData>() as u64),
        },
        count: None,
    },
    BindGroupLayoutEntry {
        binding: 10,
        visibility: ShaderStages::COMPUTE,
        ty: BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: BufferSize::new(mem::size_of::<WireId>() as u64),
        },
        count: None,
    },
];

const COMMON_SHADER_SOURCE: &str = include_str!("../shaders/common.wgsl");

macro_rules! include_shader {
    ($name:literal) => {{
        const SHADER_SOURCE: &str = include_str!(concat!("../shaders/", $name));
        const FULL_SHADER_SOURCE: &str =
            const_format::concatcp!(COMMON_SHADER_SOURCE, SHADER_SOURCE);

        ShaderModuleDescriptor {
            label: Some($name),
            source: ShaderSource::Wgsl(FULL_SHADER_SOURCE.into()),
        }
    }};
}

pub fn create_simulator(builder: SimulatorBuilder) -> Result<Simulator, ()> {
    use wgpu::util::{BufferInitDescriptor, DeviceExt};
    use wgpu::*;

    let (device, queue) = device();

    let list_data_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(slice::from_ref(&ListData::zeroed())),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
    });

    let conflict_list_buffer = device.create_buffer(&BufferDescriptor {
        label: None,
        size: (256 * mem::size_of::<WireId>()) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let wire_states = builder.wire_states.build(&device);
    let wire_drives = builder.wire_drives.build(&device);
    let wire_drivers = builder.wire_drivers.build(&device);
    let wires = builder.wires.build(&device);

    let output_states = builder.output_states.build(&device);
    let outputs = builder.outputs.build(&device);
    let inputs = builder.inputs.build(&device);
    let memory = builder.memory.build(&device);
    let components = builder.components.build(&device);

    let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: None,
        entries: BIND_GROUP_ENTRIES,
    });

    let bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: wire_states.binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: wire_drives.binding(),
            },
            BindGroupEntry {
                binding: 2,
                resource: wire_drivers.binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: wires.binding(),
            },
            BindGroupEntry {
                binding: 4,
                resource: output_states.binding(),
            },
            BindGroupEntry {
                binding: 5,
                resource: outputs.binding(),
            },
            BindGroupEntry {
                binding: 6,
                resource: inputs.binding(),
            },
            BindGroupEntry {
                binding: 7,
                resource: memory.binding(),
            },
            BindGroupEntry {
                binding: 8,
                resource: components.binding(),
            },
            BindGroupEntry {
                binding: 9,
                resource: list_data_buffer.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 10,
                resource: conflict_list_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::COMPUTE,
            range: 0..4,
        }],
    });

    let wire_shader_desc = include_shader!("wire.wgsl");
    let wire_shader = device.create_shader_module(wire_shader_desc);

    let wire_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &wire_shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    let component_shader_desc = include_shader!("component.wgsl");
    let component_shader = device.create_shader_module(component_shader_desc);

    let component_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &component_shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    let reset_shader_desc = include_shader!("reset.wgsl");
    let reset_shader = device.create_shader_module(reset_shader_desc);

    let reset_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &reset_shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    Ok(Simulator {
        device,
        queue,

        list_data_buffer,
        conflict_list_buffer,

        wire_states,
        wire_drives,
        wire_drivers,
        wires,

        output_states,
        outputs,
        inputs,
        memory,
        components,

        bind_group,
        _wire_shader: wire_shader,
        wire_pipeline,
        _component_shader: component_shader,
        component_pipeline,
        _reset_shader: reset_shader,
        reset_pipeline,

        staging_buffer: None,
        wire_states_need_sync: false,
        memory_needs_sync: false,
    })
}
