+++
date = '2026-03-18T16:58:43-06:00'
title = 'Buffer Creation'
+++
Now that we have a command buffer setup we can actually command the GPU to do something. Images are a bit more complicated, so to start with we're going to fill a buffer with RGBA data. Then, after the command buffer finishes (the wait idle we put at the end of last chapter) we write that data to a PNG file on disk.

We're going to use the `gpu-allocator` library we mentioned before to actually allocate memory for that buffer instead of manually doing so. This requires us to set-up an `Allocator` to start with:
````rust {wrap="false"}
let mut allocator = Allocator::new(&AllocatorCreateDesc {
    instance: instance.clone(),
    device: device.clone(),
    physical_device,
    debug_settings: Default::default(),
    buffer_device_address: false,
    allocation_sizes: Default::default(),
})?;
````
This requires a copy of our `instance`, `device`, and `physical_device` to actually make the Vulkan calls to allocate. `debug_settings` and `allocation_sizes` allow you to customize settings for debug logging and the sizes that the allocator uses of course. Finally, `buffer_device_address` is required to use a Vulkan feature of the same name. This feature essentially lets you get a pointer to the GPU memory and use it in shaders and other features. We don't need this for now so we'll disable it.

## Buffer Creation and Memory Binding
For ease of use we're going to set up some constants at the top of the file real quick for our image
````rust {wrap="false"}
const IMAGE_WIDTH: u32 = 1920;
const IMAGE_HEIGHT: u32 = 1080;
const IMAGE_SIZE_BYTES: vk::DeviceSize = (IMAGE_WIDTH * IMAGE_HEIGHT * 4) as vk::DeviceSize;
````
Width and height are self explanatory, feel free to set these to whatever you want. For bytes since it's an RGBA image that's one byte per color channel so 4 bytes per pixel total. Now back to our `new()` function we can create the buffer:
````rust {wrap="false"}
let buffer = device.create_buffer(
    &vk::BufferCreateInfo::default()
        .size(IMAGE_SIZE_BYTES)
        .usage(vk::BufferUsageFlags::TRANSFER_DST),
    None,
)?;
````
Very similar to how we've created everything else, only thing of note here is the usage flags. Vulkan requires you to specify how you're going to use a resource (images and buffers) when you create it so the driver can do potential optimizations. In practice usages for images are very important, but for buffers they only really affect the alignment of the allocation which you usually don't care about. This means its fine to specify almost every usage flag for buffers in practice (with the exception of `DESCRIPTOR_HEAP` and `DESCRIPTOR_BUFFER` which we will talk about soon).

In this case, we're going to use some commands in the command buffer to fill this buffer with RGBA data, that requires that our buffer has the `TRANSFER_DST` usage.

Next, we're going to use our `allocator` too actually allocate the memory for our buffer:
````rust {wrap="false"}
let buffer_allocation = allocator.allocate(&AllocationCreateDesc {
    name: "Image buffer",
    requirements: device.get_buffer_memory_requirements(buffer),
    location: MemoryLocation::CpuToGpu,
    linear: true,
    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
})?;
````
We give it a debug name, tell it to use the default allocator scheme, and give it the memory requirements (alignment from usage as mentioned) for the buffer we created. Linear matters for images and we'll talk about it in the future, but for now note buffers are always linear. 

Finally we tell it *where* we want the memory to be, we're going to mainly use `CpuToGPU` which typically means it will be in VRAM but viewable from the host (`DEVICE_LOCAL | HOST_VISIBLE` in other words). This memory is special and known as `BAR` memory, on older systems this is limited to 256 megabytes (TOTAL, not just for your program). But, on most modern systems they have a technology called `ReBAR` (resizable BAR) which allows you to view the entire VRAM from host. Note, this also uses a lot of address space so it doesn't work well on 32 bit systems, but on 64 bit systems its fine. Since we're assuming modern systems in this guide this memory type is going to be our best friend.

One thing to keep in mind about BAR memory, is that writing to it is fast because the write doesn't actually happen until its flushed. Which, you can do manually, or usually its "coherent" which means its flushed when you submit a command buffer. However, reading from it has a lot of latency because it has to receive it from VRAM then and there. For large reads this is usually fine, but for smaller sequential ones the latency is really bad and you should prefer `GpuToCpu`.

Now we can actually bind that buffer we created to this allocation
````rust {wrap="false"}
device.bind_buffer_memory(
    buffer,
    buffer_allocation.memory(),
    buffer_allocation.offset(),
)?;
````
And of course store it in our struct
````rust {wrap="false"}
pub struct Engine {
    // ...
    pub buffer: vk::Buffer,
    pub buffer_allocation: Allocation,
}

impl Engine {
    pub fn new() -> anyhow::Result<Self> {
        unsafe {
            // ...
            Ok(Self {
                // ...
                buffer,
                buffer_allocation,
            })
        }
    }
}
````
## Filling and Writing to Disk
Now in the `run()` function we can write the buffer to disk after filling it. Let's start with the harder part of writing to disk, we're going to use the `image` crate to do so. So, add that to our dependencies in `Cargo.toml`:
````toml {wrap="false"}
[dependencies]
image = "0.25.9"
````
Now after our wait idle (so the command buffer finishes executing) we can write it to disk which is quite simple:
````rust {wrap="false"}
// If we give the slice directly we run into the sequential reads from BAR memory issue we mentioned earlier, making this save very slow.
// So, to do it one large read we're going to copy it to a fully host-local vector first.
let data = self.buffer_allocation.mapped_slice().unwrap().to_vec();
image::save_buffer(
    Path::new("output.png"),
    &data,
    IMAGE_WIDTH,
    IMAGE_HEIGHT,
    image::ColorType::Rgba8,
)?;
````

Now to fill the buffer, there's a special command `fill_buffer` which allows you to fill a certain range with a `u32` value. Luckily for us color codes fit perfectly into a `u32` so we can do this:
````rust {wrap="false"}
self.device.cmd_fill_buffer(
    self.cmd,
    self.buffer,
    0,
    vk::WHOLE_SIZE,
    u32::from_be(0x0c5a81ff),
);
````
This command takes an offset and size, we want the whole buffer to be filled so we do no offset and `vk::WHOLE_SIZE` is a special value that uses whats left of the given buffer as the size. The color value we give is a nice blue. Note that color codes are in big-endian, but most computers use little endian to store values. So we use `u32::from_be` to convert the color code to little-endian. This outputs this if you run it:
![Blue](blue_output.png?width=30vw)

You can even use the offset to mix colors like this:
````rust {wrap="false"}
let half = IMAGE_SIZE_BYTES / 2;
self.device
    .cmd_fill_buffer(self.cmd, self.buffer, 0, half, u32::from_be(0x0c5a81ff));
self.device.cmd_fill_buffer(
    self.cmd,
    self.buffer,
    half,
    vk::WHOLE_SIZE,
    u32::from_be(0x81330cff),
);
````
![Mixed](mixed_output.png?width=30vw)
## Cleanup
We need to clean up everything in our `destroy` method as per usual:
````rust {wrap="false"}
pub fn destroy(mut self) -> anyhow::Result<()> {
    unsafe {
        self.device.destroy_buffer(self.buffer, None);
        self.allocator.free(self.buffer_allocation)?;
        drop(self.allocator);

        // ...
    }
}
````
The `Allocator` object is RAII based so we need to drop it, and this needs to be done before we destroy our instance and device.

## Wow, This Is Inefficient
Obviously if we tried to just use `fill_buffer` to handle all our drawing that would be horrible inefficient. We need to use the shaders we mentioned before to actually get something more programmable. However, we can't just output to our buffer directly unfortunately.

In order to use resources in shaders you need additional metadata with them called a "descriptor", which will be the focus of the next two chapters.
