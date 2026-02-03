+++
date = '2026-02-02T16:25:53-07:00'
title = 'Instance'
+++
## Context struct
We're going to encapsulate most of our objects we create then have to haul around (`Instance`, `Device`, etc.) into one `Context` struct for the sake of convenience. We're also going to handle error handling with the `anyhow` crate, feel free to use more proper error handling methods but that is not in the scope of this guide. Add to `guide`'s `Cargo.toml`:
```toml
[dependencies]
anyhow = "1.0.100"
```
Then we can set up the project like so
```rust
use ash::vk;

pub struct Context {
	_entry: ash::Entry,
	instance: ash::Instance,
}

impl Context {
	pub fn new() -> anyhow::Result<Self> {
		unsafe {
			todo!();
		}
	}

	pub fn destroy(self) -> anyhow::Result<()> {
		unsafe {
			Ok(())
		}
	}
}

fn main() -> anyhow::Result<()> {
	let context = Context::new()?;
	context.destroy()?;

	Ok(())
}
```
`ash::entry` we haven't talked about yet, it's pretty simple it just initially links to the Vulkan implementation and loads the functions we have to use before instance creation. Since it links to Vulkan we have to hold onto it. 

Also notice we have a `destroy` function that consumes `self` instead of implementing `Drop`. The reason for this is that Vulkan has more limitations on when we can destroy stuff then the `Drop` trait enforces. For example, once we start making `Buffer`'s you can't destroy buffers *after* the instance is destroyed, but the auto-running `Drop` may do so. Using consuming destroy functions lets us explicitly control the order.
## How ash maps to the Vulkan spec
To start with let's actually create the entry, which is pretty simple and self-explanatory.
```rust
let entry = ash::Entry::load()?;
```
Then, in order to create the instance we call `entry.create_instance()`, before we do that though if you look at the docs for that function you can see it links to the Vulkan spec with the function it binds to, in this case [vkCreateInstance](https://docs.vulkan.org/refpages/latest/refpages/source/vkCreateInstance.html).

Let's look at the spec for that function:
```c
// Provided by VK_VERSION_1_0
VkResult vkCreateInstance(
    const VkInstanceCreateInfo*                 pCreateInfo,
    const VkAllocationCallbacks*                pAllocator,
    VkInstance*                                 pInstance);
```
You can see the first argument is a pointer (`ash` takes it in by reference instead of course) to a `VkInstanceCreateInfo` that contains the info needed to create the instance. Then a pointer to a `VkAllocationCallbacks`, which is only used for debugging or if you need a custom host side allocation setup for some reason. If you pass null it just does the default allocator, so `ash` takes it in by `Option` (which we will pass `None` for this entire guide). Finally, the function takes in an output pointer to a `VkInstance` that it creates, and returns a `VkResult` to indicate if it errors or not. That functions pretty similar in practice to how `Result` works in Rust, so of course `ash` maps it to that.

It's very useful to understand how to read the spec relative to the bindings `ash` gives, because the spec gives in depth info about each parameter, description of what the function does, and any invalid inputs. Learning to map and read the spec is a very important skill. Luckily, Vulkan tends to follow patterns with how its functions and structs look, any `create_*` function will follow the pattern above.

Now, let's talk about that [VkInstanceCreateInfo](https://docs.vulkan.org/refpages/latest/refpages/source/VkInstanceCreateInfo.html) struct we pass in. As mentioned all structs in Vulkan follow a similar pattern, namely if you look at the spec for that struct you can see something interesting with the first two members:
```c
// Provided by VK_VERSION_1_0
typedef struct VkInstanceCreateInfo {
    VkStructureType             sType;
    const void*                 pNext;
    ...
} VkInstanceCreateInfo;
```
Vulkan is designed to be extendable in the future without causing breaking changes, the way it accomplishes this is by allowing almost every struct to be "extended" by another one by passing a pointer to it in `pNext`. What struct specifically depends on the extension or feature used. 

But there's a problem with this, with Vulkan being a C API how does it know what the type of struct you pass is? After all multiple extensions can extend a struct at once. That's where the `sType` parameter comes in, it's just a big enum value to `VkStructureType` that just says what the type is. In this case it **must** be set to `VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO`. Vulkan implementations can then cast the `pNext` pointer to either [VkBaseInStructure](https://docs.vulkan.org/refpages/latest/refpages/source/VkBaseInStructure.html) or [VkBaseOutStructure](https://docs.vulkan.org/refpages/latest/refpages/source/VkBaseOutStructure.html). 

Which both look something like this:
```c
// Provided by VK_VERSION_1_0
typedef struct VkBaseInStructure {
    VkStructureType                    sType;
    const struct VkBaseInStructure*    pNext;
} VkBaseInStructure;
```
So the implementation knows the exact type from `sType` and can follow the `pNext` chain downwards until it's null, brilliant! The way `ash` maps this is by having builder methods on structs, then the `default` method fills out that `sType` for you based on the type and zero initializes everything else. Vulkan *usually* has sane defaults for things you don't have to fill out when they're zeroed out, so this works great in practice. 

To handle `pNext` it has a `push` method that can only be called on top level structs and takes in a reference to an extension struct that it adds to the *end* of the `pNext` chain. So if the current chain looks like `A -> B -> C` and you call `A.push(&mut D)` then the chain will look like `A -> B -> C -> D`.
## Actually creating the instance
Looking at the rest of `VkInstanceCreateInfo` it looks like this:
```c
// Provided by VK_VERSION_1_0
typedef struct VkInstanceCreateInfo {
    ...
    const VkApplicationInfo*    pApplicationInfo;
    uint32_t                    enabledLayerCount;
    const char* const*          ppEnabledLayerNames;
    uint32_t                    enabledExtensionCount;
    const char* const*          ppEnabledExtensionNames;
} VkInstanceCreateInfo;
```
`VkApplicationInfo` contains info about our specific application, which we will come back to in a second. The other two parameters is a count and then pointer to a list of layer and extension names. `ash` maps these count + pointer pairs to Rust slices of course. 

Layers basically inject into function calls you make and usually run stuff on top, but they don't add any functionality themselves. For example the validation configuration we're using in `Vulkan Configurator` adds `VK_LAYER_KHRONOS_validation` as an "implicit layer", which means that it's enabled by default without the developer having to specify it. That validation layer then just makes sure everything you're doing is valid per the Vulkan spec and throws error messages otherwise. Programs like OBS or Steam also use these implicit layers to record the program or draw overlays on top.

Extensions, as briefly mentioned before actually add new structs and functions that you can use as a developer. Since these extensions are instance level they're "global" to all devices. When we get to creating devices they also have their own device-level extensions that are specific to that device and the graphics driver for it must support.

We're not using any explicit layers or extensions for now so we can leave those empty. Let's fill out the struct `ash` side so far finally
```rust
let instance = entry.create_instance(&vk::InstanceCreateInfo::default()
    .application_info(&vk::ApplicationInfo::default()),
None)?;
```
Now we can talk about the [VkApplicationInfo](https://docs.vulkan.org/refpages/latest/refpages/source/VkApplicationInfo.html) we ignored before. Looking at the spec for it:
```c
// Provided by VK_VERSION_1_0
typedef struct VkApplicationInfo {
    VkStructureType    sType;
    const void*        pNext;
    const char*        pApplicationName;
    uint32_t           applicationVersion;
    const char*        pEngineName;
    uint32_t           engineVersion;
    uint32_t           apiVersion;
} VkApplicationInfo;
```
Same `sType` and `pNext` stuff as before, the other members is mostly info that's specific to the application like name, version, engine name, etc. Some graphics drivers like to make engine or application specific optimizations, and it uses the info specified there to differentiate. We will leave this info blank for this guide, but feel free to fill it out if you wish!

The important member we'll focus on is `apiVersion`, which essentially acts like the minimum Vulkan version that the device we pick must support. For example if we set it to 1.2 then we cannot pick devices that only support Vulkan 1.1 or 1.0. We can, however, pick a device that supports 1.3 since it's above 1.2, but then we can't use any 1.3 functionality since the instance is still 1.2.

For our case we're going to require the latest Vulkan version, 1.4. So now filling out the struct should look something like below:
```rust
let instance = entry.create_instance(&vk::InstanceCreateInfo::default()
    .application_info(&vk::ApplicationInfo::default()
        .api_version(vk::API_VERSION_1_4)
    ),
None)?;
```
## Closing thoughts
That was a lot of info to go through, I won't go as in depth with each individual argument from here on. It's just important to understand how to read the C API equivalent so you're able to map it on your own.

That being said, please don't be intimidated by all the info, as mentioned Vulkan follows the patterns mentioned above so once you're used to it, it's very intuitive to write!