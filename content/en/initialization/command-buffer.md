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
## Starting and submitting the command buffer
We don't have any buffers or work we could actually do in the command buffer yet, but let's set up a bit of a framework so we can easily in future pages.

First we're going to add a `run` function to our `Engine` struct, where we fill it with commands after we finish initializing everything:
````rust {wrap="false"}
impl Engine {
    pub fn run(&self) -> anyhow::Result<()> {
        unsafe {
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
````
Now to actually fill the command buffer there's a pairing `begin` and `end` function, which is pretty self-explanatory:
````rust {wrap="false"}
self.device.begin_command_buffer(self.cmd, &vk::CommandBufferBeginInfo::default()
    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
)?;
// Commands here
self.device.end_command_buffer(self.cmd)?;
````
The only special thing we're doing here with [vk::CommandBufferBeginInfo](https://docs.vulkan.org/refpages/latest/refpages/source/VkCommandBufferBeginInfo.html) is specifying the `ONE_TIME_SUBMIT` flag which just means we promise that each recording of this command buffer will only be submitted once, and that we will reset it between submissions. We're only doing one submission for now, so no need to reset. This flag just lets graphics drivers optimize how this command buffer is dealt with once it's submitted, it doesn't affect any behavior beyond the promise.

Next, to actually submit this command buffer we'll call `device.queue_submit2(...)` notice that this has a 2 at the end. At some point a lot of old commands responsible for synchronizing work between host and device (including submitting work from command buffers to the device) were replaced with ones that were more ergonomic. That's why there's also a `queue_submit` (without the 2). These replacement functions are part of a feature called `synchronization2`, this was originally an extension but was promoted a core mandatory part of Vulkan in 1.3. Since we're using Vulkan 1.4, we can use this feature.

As mentioned previously some features need to be enabled because of potential performance impact, and this is one of those cases. To do so we just add `vk::PhysicalDeviceVulkan13Features` to the pNext chain of our logical device creation with the `synchronization2` boolean set to true to enable the feature:
````rust {wrap="false"}
let device = instance.create_device(physical_device, &vk::DeviceCreateInfo::default()
    .push(&mut vk::PhysicalDeviceVulkan13Features::default()
        .synchronization2(true)
    )
    .queue_create_infos(&[vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family)
        .queue_priorities(&[1.0])
    ]),
None)?;
````
Now, back to our `run` function we can finally submit our command buffer:
````rust {wrap="false"}
self.device.queue_submit2(self.queue, &[vk::SubmitInfo2::default()
    .command_buffer_infos(&[vk::CommandBufferSubmitInfo::default()
        .command_buffer(self.cmd)
    ])
], vk::Fence::null())?;
````
This is pretty simple, it just takes the queue we're submitting to and a list of submissions. However, if you look at the arguments for [vk::SubmitInfo2](https://docs.vulkan.org/refpages/latest/refpages/source/VkSubmitInfo2.html) and the `queue_submit2` command itself we're ignoring wait semaphores, signal semaphores, and the fence parameter. Since the device is asynchronous (and usually an entirely separate piece of hardware from the host) we are given some tools to order work between itself and the host. Semaphores allow you to synchronize GPU→GPU work, for example waiting for a previous submission before doing the next. Fences are for GPU→CPU work, you can query them or wait for them to finish from host-side.

In fact, if you run the program as is with validation layers on you'll get an error like so:
```
Validation Error: [ VUID-vkDestroyCommandPool-commandPool-00041 ] | MessageID = 0xad474cda
vkDestroyCommandPool(): (VkCommandBuffer 0x15d27c1b130) is in use.
The Vulkan spec states: All VkCommandBuffer objects allocated from commandPool must not be in the pending state (https://docs.vulkan.org/spec/latest/chapters/cmdbuffers.html#VUID-vkDestroyCommandPool-commandPool-00041)
```
"pending state" means that the device is still executing the work we submitted, so we're not allowed to destroy it until it finishes. We could use a fence to do so, but for the sake of simplicity we will just put this call right after the submission:
````rust {wrap="false"}
self.device.queue_wait_idle(self.queue)?;
````
This just blocks the host until *all* work (even across multiple submissions) on that queue is finished. Now that we have a framework, we can create a buffer and actually draw something next page!