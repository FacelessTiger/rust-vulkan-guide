use ash::vk;

pub struct Engine {
    _entry: ash::Entry,
    pub instance: ash::Instance,
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

            Ok(Self {
                _entry: entry,
                instance,
            })
        }
    }

    pub fn destroy(self) -> anyhow::Result<()> {
        unsafe {
            self.instance.destroy_instance(None);
            Ok(())
        }
    }
}

fn main() -> anyhow::Result<()> {
    let engine = Engine::new()?;
    engine.destroy()?;

    Ok(())
}