use objc2::runtime::NSObject;
use objc2::{msg_send, rc::Retained};
use objc2_io_surface::IOSurfaceRef;
use objc2_metal::{MTLPixelFormat, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage};

use foreign_types_shared::ForeignType;
use winit::dpi::PhysicalSize;

pub struct WPGPUTextureFromMetal {
    pub size: PhysicalSize<u32>,
}

impl WPGPUTextureFromMetal {
    pub fn new(size: PhysicalSize<u32>) -> Self {
        Self { size }
    }

    pub fn get(
        &self,
        wgpu_device: &wgpu::Device,
        surfman_device: &surfman::Device,
        surfman_surface: &surfman::Surface,
    ) -> wgpu::Texture {
        let objc2_metla_texture =
            self.objc2_metla_texture(wgpu_device, surfman_device, surfman_surface);

        return self.wgpu_hal_texture(wgpu_device, objc2_metla_texture);
    }

    fn create_texture_from_iosurface(
        &self,
        device: &objc2::runtime::NSObject,
        descriptor: &MTLTextureDescriptor,
        iosurface: &IOSurfaceRef,
        plane: objc2_foundation::NSUInteger,
    ) -> Option<Retained<NSObject>> {
        unsafe {
            msg_send![device, newTextureWithDescriptor:descriptor, iosurface:iosurface, plane:plane]
        }
    }

    fn objc2_metla_texture(
        &self,
        wgpu_device: &wgpu::Device,
        surfman_device: &surfman::Device,
        surfman_surface: &surfman::Surface,
    ) -> Retained<NSObject> {
        unsafe {
            let metal_device = wgpu_device.as_hal::<wgpu::wgc::api::Metal>().unwrap();
            let device_raw = metal_device.raw_device().lock().clone();

            let texture_descriptor = MTLTextureDescriptor::new();
            texture_descriptor.setDepth(1);
            texture_descriptor.setMipmapLevelCount(1);
            texture_descriptor.setSampleCount(1);
            texture_descriptor.setUsage(MTLTextureUsage::ShaderRead);
            texture_descriptor.setPixelFormat(MTLPixelFormat::RGBA8Unorm);
            texture_descriptor.setTextureType(MTLTextureType::Type2D);
            texture_descriptor.setWidth(self.size.width as usize);
            texture_descriptor.setHeight(self.size.height as usize);

            let native_surface = surfman_device.native_surface(&surfman_surface);
            let io_surface = native_surface.0;

            return self
                .create_texture_from_iosurface(
                    &*(device_raw.as_ptr() as *mut objc2::runtime::NSObject),
                    &texture_descriptor,
                    &io_surface,
                    0,
                )
                .unwrap();
        }
    }

    fn wgpu_hal_texture(
        &self,
        wgpu_device: &wgpu::Device,
        metal_texture: Retained<NSObject>,
    ) -> wgpu::Texture {
        unsafe {
            let ptr: *mut objc2_foundation::NSObject = Retained::into_raw(metal_texture);

            let metal_texture = metal::Texture::from_ptr(ptr as *mut _);

            let hal_texture = wgpu::hal::metal::Device::texture_from_raw(
                metal_texture,
                wgpu::wgt::TextureFormat::Rgba8Unorm,
                metal::MTLTextureType::D2,
                0,
                0,
                wgpu::hal::CopyExtent {
                    width: self.size.width,
                    height: self.size.height,
                    depth: 0,
                },
            );

            let wgpu_descriptor = wgpu::TextureDescriptor {
                label: Some("Metal IOSurface Texture"),
                size: wgpu::Extent3d {
                    width: self.size.width,
                    height: self.size.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            };

            return wgpu_device
                .create_texture_from_hal::<wgpu::wgc::api::Metal>(hal_texture, &wgpu_descriptor);
        };
    }
}
