use ash::vk;
use ash::vk::TaggedStructure;
use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{
    Allocation, AllocationCreateDesc, AllocationScheme, Allocator, AllocatorCreateDesc,
};
use std::path::Path;

const IMAGE_WIDTH: u32 = 1920;
const IMAGE_HEIGHT: u32 = 1080;
const IMAGE_SIZE_BYTES: vk::DeviceSize = (IMAGE_WIDTH * IMAGE_HEIGHT * 4) as vk::DeviceSize;

pub struct Engine {
    _entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub allocator: Allocator,

    pub queue: vk::Queue,
    pub queue_family: u32,

    pub buffer: vk::Buffer,
    pub buffer_allocation: Allocation,

    pub command_pool: vk::CommandPool,
    pub cmd: vk::CommandBuffer,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            let entry = ash::Entry::load()?;
            let instance = entry.create_instance(
                &vk::InstanceCreateInfo::default().application_info(
                    &vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_4),
                ),
                None,
            )?;

            let physical_device = instance
                .enumerate_physical_devices()?
                .into_iter()
                .min_by_key(|physical_device| {
                    let properties = instance.get_physical_device_properties(*physical_device);
                    match properties.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                        _ => 2,
                    }
                })
                .ok_or(anyhow::anyhow!("No physical devices available"))?;
            let queue_family = instance
                .get_physical_device_queue_family_properties(physical_device)
                .into_iter()
                .position(|properties| {
                    properties.queue_flags.contains(
                        vk::QueueFlags::GRAPHICS
                            | vk::QueueFlags::COMPUTE
                            | vk::QueueFlags::TRANSFER,
                    )
                })
                .ok_or(anyhow::anyhow!("No main queue available"))?
                as u32;

            let device = instance.create_device(
                physical_device,
                &vk::DeviceCreateInfo::default()
                    .push(&mut vk::PhysicalDeviceVulkan13Features::default().synchronization2(true))
                    .queue_create_infos(&[vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(queue_family)
                        .queue_priorities(&[1.0])]),
                None,
            )?;
            let queue = device.get_device_queue(queue_family, 0);

            let mut allocator = Allocator::new(&AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device,
                debug_settings: Default::default(),
                buffer_device_address: false,
                allocation_sizes: Default::default(),
            })?;

            let buffer = device.create_buffer(
                &vk::BufferCreateInfo::default()
                    .size(IMAGE_SIZE_BYTES)
                    .usage(vk::BufferUsageFlags::TRANSFER_DST),
                None,
            )?;
            let buffer_allocation = allocator.allocate(&AllocationCreateDesc {
                name: "Image buffer",
                requirements: device.get_buffer_memory_requirements(buffer),
                location: MemoryLocation::CpuToGpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;
            device.bind_buffer_memory(
                buffer,
                buffer_allocation.memory(),
                buffer_allocation.offset(),
            )?;

            let command_pool = device.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(queue_family),
                None,
            )?;
            let cmd = device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(command_pool)
                    .command_buffer_count(1),
            )?[0];

            Ok(Self {
                _entry: entry,
                instance,
                physical_device,
                device,
                allocator,
                queue,
                queue_family,
                buffer,
                buffer_allocation,
                command_pool,
                cmd,
            })
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        unsafe {
            let half = IMAGE_SIZE_BYTES / 2;

            self.device.begin_command_buffer(
                self.cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
            self.device
                .cmd_fill_buffer(self.cmd, self.buffer, 0, half, u32::from_be(0x0c5a81ff));
            self.device.cmd_fill_buffer(
                self.cmd,
                self.buffer,
                half,
                vk::WHOLE_SIZE,
                u32::from_be(0x81330cff),
            );
            self.device.end_command_buffer(self.cmd)?;

            self.device.queue_submit2(
                self.queue,
                &[vk::SubmitInfo2::default().command_buffer_infos(&[
                    vk::CommandBufferSubmitInfo::default().command_buffer(self.cmd),
                ])],
                vk::Fence::null(),
            )?;
            self.device.queue_wait_idle(self.queue)?;

            let data = self.buffer_allocation.mapped_slice().unwrap().to_vec();
            image::save_buffer(
                Path::new("output.png"),
                &data,
                IMAGE_WIDTH,
                IMAGE_HEIGHT,
                image::ColorType::Rgba8,
            )?;
            Ok(())
        }
    }

    pub fn destroy(mut self) -> anyhow::Result<()> {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
            self.allocator.free(self.buffer_allocation)?;
            drop(self.allocator);

            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
            Ok(())
        }
    }
}

fn main() -> anyhow::Result<()> {
    let engine = Engine::new()?;
    engine.run()?;
    engine.destroy()?;

    Ok(())
}
