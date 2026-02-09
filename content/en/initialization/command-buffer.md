+++
date = '2026-02-04T14:29:17-07:00'
title = 'Command Buffer'
+++
After the info dump the last two pages were, we can do a quick one and finally create the command buffer we mentioned all the way back in the introduction chapter!

## Command pool creation
Similar to queue families, you create command buffers from a pool object you have to create first. Command pools also control the allocation for the backing command buffers, which means if you want to fill out multiple command buffers from different threads then you need a command pool per thread. Otherwise, you'd be modifying the same memory from multiple threads (unless you do locking which would defeat the purpose).

As usual, we have a `device.create_command_pool()` function that takes a [vk::CommandPoolCreateInfo](https://docs.vulkan.org/refpages/latest/refpages/source/VkCommandPoolCreateInfo.html). We're not interested in any flags, so the only argument we need to fill is which queue family we're going to submit command buffers from this pool to. One notable flag to mention is `vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER`, normally to reset a command buffer (delete all the GPU commands you put in it) you have to reset the entire pool. That flag enables you to reset individual command buffers from this pool rather than the whole thing, however supposedly this causes issues with some drivers trying to allocate it. So, it's recommended to just create a separate pool even if you only have one buffer per pool.
````rust {wrap="false"}
let command_pool = device.create_command_pool(&vk::CommandPoolCreateInfo::default()
    .queue_family_index(queue_family),
None)?;
````
## Command buffer allocation
Now, we just need to allocate the command buffer itself. We do this through `device.allocate_command_buffers()`, which takes its arguments through the [vk::CommandBufferAllocateInfo](https://docs.vulkan.org/refpages/latest/refpages/source/VkCommandBufferAllocateInfo.html) struct. This takes the command pool to allocate from, and how many to allocate (which we're just doing one for now). But, it also has a `level` member, this allows you to execute other command buffers. Primary command buffers can execute secondary ones, but the secondary ones have to be pre-filled of course. This seems great in theory, but in practice pre-filling command buffers like that is very difficult for anything but a static scene and command buffer filling is very quick by design. Luckily primary has a value of 0, so we can just ignore that field.
````rust {wrap="false"}
// returns a vector of the command buffers, so take the 0th index since we're making one
let cmd = device.allocate_command_buffers(&vk::CommandBufferAllocateInfo::default()
    .command_pool(command_pool)
    .command_buffer_count(1)
)?[0];
````
## Store and cleanup
Like always we need to store out new objects in the `Engine` struct and clean them up. Which is pretty simple.
````rust {wrap="false"}
pub struct Engine {
    // ...
    pub command_pool: vk::CommandPool,
    pub cmd: vk::CommandBuffer,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            // ...
            Ok(Self {
                _entry: entry,
                instance, physical_device, device,
                queue, queue_family,
                command_pool, cmd,
            })
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
````
Since the command pool is the backing memory for the buffer, we only need to destroy it.