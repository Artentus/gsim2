use crate::logic::LogicStateAtom;
use bytemuck::{Pod, Zeroable};
use std::cmp;
use std::fmt;
use std::marker::PhantomData;

pub trait BufferState {}

pub struct Building;
impl BufferState for Building {}

pub struct Finalized {
    gpu_buffer: wgpu::Buffer,
    requires_update: bool,
}
impl BufferState for Finalized {}

#[derive(Debug, Clone)]
pub enum BufferPushError {
    OutOfMemory,
}

#[repr(transparent)]
pub struct Index<Marker: ?Sized + 'static> {
    value: u32,
    _marker: PhantomData<&'static Marker>,
}

impl<Marker: ?Sized + 'static> Index<Marker> {
    pub const INVALID: Self = Self {
        value: u32::MAX,
        _marker: PhantomData,
    };

    #[inline]
    const fn new(value: u32) -> Option<Self> {
        if value == u32::MAX {
            None
        } else {
            Some(Self {
                value,
                _marker: PhantomData,
            })
        }
    }

    #[inline]
    pub const fn is_invalid(self) -> bool {
        self.value == u32::MAX
    }

    #[inline]
    const fn get(self) -> Option<u32> {
        if self.is_invalid() {
            None
        } else {
            Some(self.value)
        }
    }
}

impl<Marker: ?Sized + 'static> fmt::Debug for Index<Marker> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_invalid() {
            write!(f, "<INVALID>")
        } else {
            fmt::Debug::fmt(&self.value, f)
        }
    }
}

impl<Marker: ?Sized + 'static> Default for Index<Marker> {
    #[inline]
    fn default() -> Self {
        Self::INVALID
    }
}

impl<Marker: ?Sized + 'static> Clone for Index<Marker> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            _marker: PhantomData,
        }
    }
}

impl<Marker: ?Sized + 'static> Copy for Index<Marker> {}

impl<Marker: ?Sized + 'static> PartialEq for Index<Marker> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<Marker: ?Sized + 'static> Eq for Index<Marker> {}

impl<Marker: ?Sized + 'static> PartialOrd for Index<Marker> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<Marker: ?Sized + 'static> Ord for Index<Marker> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.value.cmp(&other.value)
    }
}

unsafe impl<Marker: ?Sized + 'static> Zeroable for Index<Marker> {}

unsafe impl<Marker: ?Sized + 'static> Pod for Index<Marker> {}

pub struct Buffer<T: Pod + 'static, S: BufferState> {
    data: Vec<T>,
    state: S,
}

impl<T: Pod + 'static, S: BufferState> Buffer<T, S> {
    #[inline]
    pub fn len(&self) -> u32 {
        self.data.len() as u32
    }

    #[inline]
    pub fn get(&self, index: Index<T>) -> Option<&T> {
        let index = index.get()? as usize;
        self.data.get(index)
    }

    #[inline]
    pub fn iter_indices(&self) -> impl Iterator<Item = Index<T>> {
        (0..self.len()).map(|index| Index::new(index).unwrap())
    }
}

impl<T: fmt::Debug + Pod + 'static, S: BufferState> fmt::Debug for Buffer<T, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.data, f)
    }
}

impl<T: Pod + 'static> Buffer<T, Building> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            state: Building,
        }
    }

    #[inline]
    pub fn get_mut(&mut self, index: Index<T>) -> Option<&mut T> {
        let index = index.get()? as usize;
        self.data.get_mut(index)
    }

    #[inline]
    pub fn push(&mut self, value: T) -> Result<Index<T>, BufferPushError> {
        let index: u32 = self
            .data
            .len()
            .try_into()
            .map_err(|_| BufferPushError::OutOfMemory)?;

        let index = Index::new(index).ok_or(BufferPushError::OutOfMemory)?;
        self.data.push(value);
        Ok(index)
    }

    pub fn build(self, device: &wgpu::Device) -> Buffer<T, Finalized> {
        use wgpu::util::{BufferInitDescriptor, DeviceExt};
        use wgpu::BufferUsages;

        let data = if self.data.len() > 0 {
            self.data.as_slice()
        } else {
            &[T::zeroed()]
        };

        let state = Finalized {
            gpu_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            }),
            requires_update: false,
        };

        Buffer {
            data: self.data,
            state,
        }
    }
}

impl<T: Pod + 'static> Default for Buffer<T, Building> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Pod + 'static> Buffer<T, Finalized> {
    #[inline]
    pub fn get_mut(&mut self, index: Index<T>) -> Option<&mut T> {
        let index = index.get()? as usize;
        self.state.requires_update = true;
        self.data.get_mut(index)
    }

    #[inline]
    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.state.gpu_buffer.slice(..)
    }

    #[inline]
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        self.state.gpu_buffer.as_entire_binding()
    }

    #[inline]
    pub fn update(&mut self, queue: &wgpu::Queue) {
        if self.state.requires_update {
            queue.write_buffer(&self.state.gpu_buffer, 0, bytemuck::cast_slice(&self.data));
            self.state.requires_update = false;
        }
    }
}

#[repr(transparent)]
pub struct Offset<Marker: ?Sized + 'static>(Index<Marker>);

impl<Marker: ?Sized + 'static> Offset<Marker> {
    pub const INVALID: Self = Self(Index::INVALID);

    #[inline]
    const fn new(value: u32) -> Option<Self> {
        match Index::new(value) {
            Some(index) => Some(Self(index)),
            None => None,
        }
    }

    #[inline]
    pub const fn is_invalid(self) -> bool {
        self.0.is_invalid()
    }

    #[inline]
    const fn get(self) -> Option<u32> {
        self.0.get()
    }
}

impl<Marker: ?Sized + 'static> fmt::Debug for Offset<Marker> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl<Marker: ?Sized + 'static> Default for Offset<Marker> {
    #[inline]
    fn default() -> Self {
        Self::INVALID
    }
}

impl<Marker: ?Sized + 'static> Clone for Offset<Marker> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<Marker: ?Sized + 'static> Copy for Offset<Marker> {}

impl<Marker: ?Sized + 'static> PartialEq for Offset<Marker> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<Marker: ?Sized + 'static> Eq for Offset<Marker> {}

impl<Marker: ?Sized + 'static> PartialOrd for Offset<Marker> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<Marker: ?Sized + 'static> Ord for Offset<Marker> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

unsafe impl<Marker: ?Sized + 'static> Zeroable for Offset<Marker> {}

unsafe impl<Marker: ?Sized + 'static> Pod for Offset<Marker> {}

pub struct LogicStateBuffer<Marker: ?Sized + 'static, S: BufferState> {
    data: Vec<LogicStateAtom>,
    state: S,
    _marker: PhantomData<&'static Marker>,
}

impl<Marker: ?Sized + 'static, S: BufferState> LogicStateBuffer<Marker, S> {
    #[inline]
    pub fn len(&self) -> u32 {
        self.data.len() as u32
    }

    #[inline]
    pub fn get(&self, offset: Offset<Marker>, count: u32) -> Option<&[LogicStateAtom]> {
        let start = offset.get()? as usize;
        let end = start + (count as usize);
        self.data.get(start..end)
    }
}

impl<Marker: ?Sized + 'static, S: BufferState> fmt::Debug for LogicStateBuffer<Marker, S> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.data, f)
    }
}

impl<Marker: ?Sized + 'static> LogicStateBuffer<Marker, Building> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            state: Building,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn get_mut(&mut self, offset: Offset<Marker>, count: u32) -> Option<&mut [LogicStateAtom]> {
        let start = offset.get()? as usize;
        let end = start + (count as usize);
        self.data.get_mut(start..end)
    }

    #[inline]
    pub fn push(&mut self, count: u32) -> Result<Offset<Marker>, BufferPushError> {
        let offset: u32 = self
            .data
            .len()
            .try_into()
            .map_err(|_| BufferPushError::OutOfMemory)?;

        let new_len = offset
            .checked_add(count)
            .ok_or(BufferPushError::OutOfMemory)?;

        let offset = Offset::new(offset).ok_or(BufferPushError::OutOfMemory)?;
        self.data.resize(new_len as usize, LogicStateAtom::HIGH_Z);
        Ok(offset)
    }

    pub fn build(self, device: &wgpu::Device) -> LogicStateBuffer<Marker, Finalized> {
        use wgpu::util::{BufferInitDescriptor, DeviceExt};
        use wgpu::BufferUsages;

        let data = if self.data.len() > 0 {
            self.data.as_slice()
        } else {
            &[LogicStateAtom::HIGH_Z]
        };

        let state = Finalized {
            gpu_buffer: device.create_buffer_init(&BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(data),
                usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
            }),
            requires_update: false,
        };

        LogicStateBuffer {
            data: self.data,
            state,
            _marker: PhantomData,
        }
    }
}

impl<Marker: ?Sized + 'static> Default for LogicStateBuffer<Marker, Building> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<Marker: ?Sized + 'static> LogicStateBuffer<Marker, Finalized> {
    #[inline]
    pub fn get_mut(&mut self, offset: Offset<Marker>, count: u32) -> Option<&mut [LogicStateAtom]> {
        let start = offset.get()? as usize;
        let end = start + (count as usize);
        self.state.requires_update = true;
        self.data.get_mut(start..end)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.data.fill(LogicStateAtom::HIGH_Z);
        self.state.requires_update = true;
    }

    #[inline]
    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.state.gpu_buffer.slice(..)
    }

    #[inline]
    pub fn binding(&self) -> wgpu::BindingResource<'_> {
        self.state.gpu_buffer.as_entire_binding()
    }

    #[inline]
    pub fn update(&mut self, queue: &wgpu::Queue) {
        if self.state.requires_update {
            queue.write_buffer(&self.state.gpu_buffer, 0, bytemuck::cast_slice(&self.data));
            self.state.requires_update = false;
        }
    }

    #[inline]
    pub fn sync(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        staging_buffer: &mut Option<wgpu::Buffer>,
    ) {
        crate::gpu::read_buffer(
            &self.state.gpu_buffer,
            &mut self.data,
            device,
            queue,
            staging_buffer,
        );
    }
}
