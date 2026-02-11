use ash::vk;
use ash::vk::TaggedStructure;

pub struct Engine {
    _entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,

    pub queue: vk::Queue,
    pub queue_family: u32,

    pub command_pool: vk::CommandPool,
    pub cmd: vk::CommandBuffer,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            let entry = ash::Entry::load()?;
            let instance = entry.create_instance(&vk::InstanceCreateInfo::default()
                .application_info(&vk::ApplicationInfo::default()
                    .api_version(vk::API_VERSION_1_4)
                ),
            None)?;

            let physical_device = instance
                .enumerate_physical_devices()?
                .into_iter()
                .min_by_key(|physical_device| {
                    match instance.get_physical_device_properties(*physical_device).device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                        _ => 3,
                    }
                })
                .ok_or(anyhow::anyhow!("No physical devices available"))?;
            let queue_family = instance
                .get_physical_device_queue_family_properties(physical_device)
                .into_iter()
                .position(|properties| {
                    properties.queue_flags.contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER)
                })
                .ok_or(anyhow::anyhow!("No main queue available"))? as u32;

            let device = instance.create_device(physical_device, &vk::DeviceCreateInfo::default()
                .push(&mut vk::PhysicalDeviceVulkan13Features::default()
                    .synchronization2(true)
                )
                .queue_create_infos(&[vk::DeviceQueueCreateInfo::default()
                    .queue_family_index(queue_family)
                    .queue_priorities(&[1.0])
                ]),
            None)?;
            let queue = device.get_device_queue(queue_family, 0);

            let command_pool = device.create_command_pool(&vk::CommandPoolCreateInfo::default()
                .queue_family_index(queue_family),
            None)?;
            let cmd = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .command_buffer_count(1)
            )?[0];

            Ok(Self {
                _entry: entry,
                instance, physical_device, device,
                queue, queue_family,
                command_pool, cmd,
            })
        }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        unsafe {
            self.device.begin_command_buffer(self.cmd, &vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            )?;
            // Commands here
            self.device.end_command_buffer(self.cmd)?;

            self.device.queue_submit2(self.queue, &[vk::SubmitInfo2::default()
                .command_buffer_infos(&[vk::CommandBufferSubmitInfo::default()
                    .command_buffer(self.cmd)
                ])
            ], vk::Fence::null())?;
            self.device.queue_wait_idle(self.queue)?;
            Ok(())
        }
    }

    pub fn destroy(self) -> anyhow::Result<()> {
        unsafe {
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