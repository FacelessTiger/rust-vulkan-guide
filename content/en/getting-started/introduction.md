+++
date = '2026-02-02T13:52:56-07:00'
title = 'Introduction'
+++

## Who this guide is for
This guide aims to target people who are proficient in the Rust programming language, and are at least a bit familiar with the basics of low level programming (at least with how pointers work) and want to work with low level modern GPU programming. But this assumes no knowledge with how graphics APIs or GPUs work in general and aims to fill you in from scratch to use the Vulkan API.

This guide also aims to use very, *very* modern features, both for the sake of speed and developer ease. That being said, almost every feature used are part of the [2026 Vulkan roadmap](https://docs.vulkan.org/spec/latest/appendices/roadmap.html#roadmap-2026) which means that mid-to-high end devices released in 2026 and onwards are expected to support all those features.

{{% expand title="Side note on one feature" %}}
One of the features we use, [descriptor heaps](https://docs.vulkan.org/features/latest/features/proposals/VK_EXT_descriptor_heap.html) is not part of the roadmap. But it officially deprecates the old method and is expected to become part of a future roadmap or Vulkan version once it's been tested enough. As of writing this article though, it's new enough that it's only available on the [beta Vulkan drivers for NVIDIA](https://developer.nvidia.com/vulkan-driver) and not available on public drivers yet.
{{% /expand %}}
## What is Vulkan anyway?
Vulkan is a low level graphics API that allows the developer immense control over the GPU. It was made in response to issues with older API's, namely OpenGL and DirectX11 for causing uncontrollable frame hitching that developers couldn't control or fix (without target specific hacks). Vulkan itself, is simply a spec that details the API itself and the expected behavior. But, the actual implementation is made by the graphics driver, **not** Vulkan (or its spec maintainer, Khronos). As it's an open spec, different companies can propose their own extensions to it and Khronos itself is simply a consortium of a bunch of companies (NVIDIA, AMD, Apple, etc.).

Vulkan can be used in any way to program a GPU, while we'll mainly be focusing on graphics and real time rendering in this guide you can also use it for compute applications like physics simulations or AI. If you want to communicate with the GPU in any way, Vulkan enables you to do so.
## High level overview and terminology
Let's quickly cover some terminology we'll refer to throughout most of this guide
* Vendor - This is a developer of the graphics driver a user is using, and thus the actual implementation of Vulkan. That is, NVIDIA, AMD, Intel, Qualcomm, etc.
* Device - Essentially just a fancy name for the GPU. Vulkan prefers using "device" and "host" since it covers integrated GPUs or unified architecture like on mobile, while GPU implies a separate discrete card.
* Host - Fancy name for the CPU, for reasons described above.

Now to try to form a high level mental image I think it's best to work backwards. Our end goal is to do some work with the device, that is we want to command it to do something. Vulkan likes to group device commands together rather than sending it one at the time for the sake of efficiency. Those commands are grouped together into a buffer, a **command buffer**. That buffer of device commands is finally submitted to a queue once we finish making it to actually ask the device to do work. 

Queues are of course local to the device that we actually want to submit the work to. Now, Vulkan has two separated but related objects for the device, `PhysicalDevice` and `Device` (the latter is sometimes called a "logical device"). The `PhysicalDevice` just contains info that you can query. That is type of device (discrete, integrated, emulated, etc.), how much memory it has, what extensions it may support, etc. While the `Device` contains the actual functions you call to act on it, and the queues to submit work to it. The reason for this separation is so you can query which device you want to pick if there's multiple options before committing to one.

Finally, before you can even query you need to set up an `Instance`, this just encapsulates "global" info that could apply to multiple devices. Like say features for interacting with or drawing to a window on the desktop.

So to summarize, from back to front the objects we'll have to create is:
* `CommandBuffer` - buffer of commands we want the device to do.
* `Queue` - where we actually submit those commands to be enqueued.
* `Device` - a device we can actually logically work with and use functions with.
* `PhysicalDevice` - just contains info we query before we choose one to make the logical device.
* `Instance` - "global" multiple-device info.

This overview glosses over some details that we'll go over as we get to it, but this is a nice high level model to understand the next chapter.