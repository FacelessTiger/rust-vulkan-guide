+++
date = '2026-02-03T12:48:29-07:00'
title = 'Device and Queue'
+++
## Selecting the physical device
Now that the `vk::Instance` is created we need to actually choose the physical device, as discussed in the high level overview in the introduction. We can just call `instance.enumerate_physical_devices()` to get a vector of the possible devices. Then, we can use `instance.get_physical_device_properties(physical_device)` to get a `vk::PhysicalDeviceProperties` for each one that contains info about what type of device it is, what Vulkan version it supports, how much memory it has, etc. 

For this guide, we're going to use a very simple algorithm:
````rust {wrap="false"}
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
````
This will prioritize discrete, over integrated, over any other type of device and just choose the first one if there's a tie. Or, if the vector is empty that means that no device supports Vulkan, so we can just return an error. This won't cover all cases, for example it's possible a user has two discrete GPUs, and we select one that has much lower features and thus fails logical device creation while the other one wouldn't. But, this is good enough for 99% of cases.

Feel free to add more criteria or a scoring system or whatever else you wish if you want more complex physical device selection.
## Queue family selection
On top of the commands we put in the command buffer mentioned before, there's also a way to explicitly program the GPU much the same way you do the CPU. You use something called shaders that you "bind" then commands afterward refer to them. 

Shaders are split into two main types:
* Graphics shaders - Which uses hardware accelerated rasterizer, any `cmd_draw*` command uses the bound graphics shader. These have quite a few parameters, and we'll get into what a "rasterizer" is later, for now we'll focus on compute shaders.
* Compute shaders - The GPU is designed to do heavily parallelized work, and so a compute shader is essentially a function that is run by an amount of "threads" you specify. It's similar to spawning a bunch of threads to run the same function on the CPU (with the only argument difference being the thread index), except while your CPU only has maybe 4-32 threads a GPU has thousands to millions. These correlate to a `cmd_dispatch*` command that helps to specify how many threads to run for the bound compute shader.

One thing that we left out of the high level overview is that queues are organized together into "queue families" based off of capabilities. Those capabilities are indicated by  [VkQueueFlagBits](https://docs.vulkan.org/refpages/latest/refpages/source/VkQueueFlagBits.html). There's a couple capabilities there but the one's we are interested in is `GRAPHICS`, `COMPUTE`, and `TRANSFER`.
* `GRAPHICS` - enables usage of graphics shaders and the related commands.
* `COMPUTE` - enables usage of compute shaders and the related commands.
* `TRANSFER` - enables usage of commands to copy buffers or images to each other.

Queue families can have multiple of these capabilities together, and indeed both `GRAPHICS` and `COMPUTE` imply `TRANSFER` capability since you can copy buffers and images in shaders. Notably, for a long time even though they implied `TRANSFER` it was optional for the implementation to advertise it being enabled on that queue family until Vulkan 1.4 added the following requirement:
```
All queues supporting VK_QUEUE_GRAPHICS_BIT or VK_QUEUE_COMPUTE_BIT must also advertise VK_QUEUE_TRANSFER_BIT.
```
Combine that with the requirement that implementors had since Vulkan 1.0:
```
If an implementation exposes any queue family that supports graphics operations, at least one queue family of at least one physical device exposed by the implementation must support both graphics and compute operations.
```
And that means we have a guaranteed queue family that supports all three capabilities, generally this is queue family 0 but there's no guarantee for that so we'll find it manually. 

Note that generally the fewer capabilities a queue family has, the more specialized the hardware probably is for it. For example a queue family with `TRANSFER` capability and none of the other two usually indicates that queue family uses DMA (direct memory access) hardware. But, for the duration of this guide we'll keep things simple and stick to using one queue from the queue family with all three capabilities.

Finding this queue family is pretty similar to physical device selection, just using `instance.get_physical_device_queue_family_properties(physical_device)` to get a list of queue family properties, checking for the first queue family that supports all three capabilities, then returning the index. There is no object for queue families, they're just referred to by the index in that list.
````rust {wrap="false"}
let queue_family = instance
    .get_physical_device_queue_family_properties(physical_device)
    .into_iter()
    .position(|properties| {
        properties.queue_flags.contains(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER)
    })
    .ok_or(anyhow::anyhow!("No main queue available"))? as u32;
````
Since it's an index Rust will return the position as a `usize`, but Vulkan refers to it with a `u32` so we'll need to cast it.
## Logical device creation
Just like with the instance, we're going to use `instance.create_device()` to make the logical device, and it takes a `vk::DeviceCreateInfo` for the info needed to create it. Let's look at the C version of the struct to see the arguments:
````c {wrap="false"}
// Provided by VK_VERSION_1_0
typedef struct VkDeviceCreateInfo {
    VkStructureType                    sType;
    const void*                        pNext;
    VkDeviceCreateFlags                flags;
    uint32_t                           queueCreateInfoCount;
    const VkDeviceQueueCreateInfo*     pQueueCreateInfos;
    // enabledLayerCount is legacy and should not be used
    uint32_t                           enabledLayerCount;
    // ppEnabledLayerNames is legacy and should not be used
    const char* const*                 ppEnabledLayerNames;
    uint32_t                           enabledExtensionCount;
    const char* const*                 ppEnabledExtensionNames;
    const VkPhysicalDeviceFeatures*    pEnabledFeatures;
} VkDeviceCreateInfo;
````
We're not using any special flags, so let's focus on `pQueueCreateInfos` first. We need to specify what queues we're going to use when creating the logical device, the `VkDeviceQueueCreateInfo` struct specifies a queue family, how many queues we're going to use in that family, and a list of priority values (from 0.0 - 1.0) that hints to the implementation that some queues have higher priority than others and thus should be allotted more processing time. Since we're only using one queue the priority doesn't matter.

Next is layers, as noted in the comments these are deprecated. It used to be that instance level layers were able to intercept the "global" functions like queue family enumeration and device properties. While device layers were able to intercept device level functions like command buffer creation and submission. But, every layer just ended up allowing you to specify for both because it's more useful to control everything that way (like again with validation layers). So, instance layers were expanded to be able to intercept all functions and device layers deprecated.

Extensions as mentioned before actually add new functionality (structs and functions) instead of just intercepting. Device level extensions have to be supported by the graphics driver, the Vulkan SDK has a nice tool called the `Vulkan Hardware Capability Viewer` that lets you view the queue families and possible extensions/features your device supports. For now, we're not going to use any extensions so we'll leave this blank.

Finally, `pEnabledFeatures`. Some features are optional for the implementation to support, some are exclusive with each other, and some introduce some performance overhead just by being enabled. So, Vulkan requires you to explicitly enable a couple features. The issue though is that `VkPhysicalDeviceFeatures` only covers features in Vulkan 1.0. So, it's since been deprecated, and instead you attach a `VkPhysicalDeviceFeatures2` struct to `pNext`, and every extension and Vulkan version after 1.0 has its own feature struct you add to the `pNext` chain. For example, Vulkan 1.1 has its own `VkPhysicalDeviceVulkan11Features` features struct. Again, we're not interested in enabling any features for now so we don't attach anything to `pNext`.

Now we can actually create it, only thing we're filling out is the queue create info, everything else we're leaving zeroed.
````rust {wrap="false"}
let device = instance.create_device(physical_device, &vk::DeviceCreateInfo::default()
    .queue_create_infos(&[vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family)
        .queue_priorities(&[1.0]) // this needs to be the same size as queue count, so ash sets both when we do  this
    ]),
None)?;
````
Now we can get the Queue object we've been working for, which anti climactically is a one-liner:
````rust {wrap="false"}
let queue = device.get_device_queue(queue_family, 0); // get queue index 0 in the queue family we chose earlier
````
## Storing in the struct and cleanup
Let's modify our `Engine` struct real quick to store all these objects we created:
````rust {wrap="false"}
pub struct Engine {
    _entry: ash::Entry,
    pub instance: ash::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,

    pub queue: vk::Queue,
    pub queue_family: u32,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        // ...
        
        Ok(Self {
            _entry: entry,
            instance, physical_device, device,
            queue, queue_family,
        })
    }
}
````
Now if you run this with the `Vulkan Configurator` open in the background, you'll get a validation error about not destroying the device. As mentioned before, you have to manually cleanup things in Vulkan and in this case we need to destroy the logical device *before* the instance.
````rust {wrap="false"}
pub fn destroy(self) -> anyhow::Result<()> {
    unsafe {
        self.device.destroy_device(None);
        self.instance.destroy_instance(None);
        Ok(())
    }
}
````