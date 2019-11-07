use std::time::Duration;

use rusb::{
    AsyncGroup, Context, Device, DeviceDescriptor, DeviceHandle, Direction, Result, Transfer,
    TransferType,
};

#[derive(Debug)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8,
}

fn main() {
    let vid: u16 = 0x1A86;
    let pid: u16 = 0xE024;

    let context = Context::new().unwrap();

    let (device, device_desc, mut handle) =
        open_device(&context, vid, pid).expect("Could not open device");
    read_device(&context, &device, &device_desc, &mut handle).expect("barfed here");
}

fn open_device(
    context: &Context,
    vid: u16,
    pid: u16,
) -> Option<(Device, DeviceDescriptor, DeviceHandle)> {
    let devices = match context.devices() {
        Ok(d) => d,
        Err(_) => return None,
    };

    for device in devices.iter() {
        let device_desc = match device.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };

        if device_desc.vendor_id() == vid && device_desc.product_id() == pid {
            match device.open() {
                Ok(handle) => return Some((device, device_desc, handle)),
                Err(_) => continue,
            }
        }
    }

    None
}

fn read_device(
    context: &Context,
    device: &Device,
    device_desc: &DeviceDescriptor,
    handle: &mut DeviceHandle,
) -> Result<()> {
    match find_readable_endpoint(device, device_desc, TransferType::Interrupt) {
        Some(endpoint) => read_endpoint(context, handle, endpoint, TransferType::Interrupt),
        None => println!("No readable interrupt endpoint"),
    }

    match find_readable_endpoint(device, device_desc, TransferType::Bulk) {
        Some(endpoint) => read_endpoint(context, handle, endpoint, TransferType::Bulk),
        None => println!("No readable bulk endpoint"),
    }

    Ok(())
}

fn find_readable_endpoint(
    device: &Device,
    device_desc: &DeviceDescriptor,
    transfer_type: TransferType,
) -> Option<Endpoint> {
    for n in 0..device_desc.num_configurations() {
        let config_desc = match device.config_descriptor(n) {
            Ok(c) => c,
            Err(_) => continue,
        };

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                for endpoint_desc in interface_desc.endpoint_descriptors() {
                    if endpoint_desc.direction() == Direction::In
                        && endpoint_desc.transfer_type() == transfer_type
                    {
                        return Some(Endpoint {
                            config: config_desc.number(),
                            iface: interface_desc.interface_number(),
                            setting: interface_desc.setting_number(),
                            address: endpoint_desc.address(),
                        });
                    }
                }
            }
        }
    }

    None
}

fn read_endpoint(
    context: &Context,
    handle: &mut DeviceHandle,
    endpoint: Endpoint,
    transfer_type: TransferType,
) {
    println!("Reading from endpoint: {:?}", endpoint);

    configure_endpoint(handle, &endpoint).unwrap();

    let mut buffers = [[0u8; 128]; 8];

    {
        let mut async_group = AsyncGroup::new(context);
        let timeout = Duration::from_secs(1);

        match transfer_type {
            TransferType::Interrupt => {
                for buf in &mut buffers {
                    async_group
                        .submit(Transfer::interrupt(handle, endpoint.address, buf, timeout))
                        .unwrap();
                }
            }
            TransferType::Bulk => {
                for buf in &mut buffers {
                    async_group
                        .submit(Transfer::bulk(handle, endpoint.address, buf, timeout))
                        .unwrap();
                }
            }
            _ => unimplemented!(),
        }

        loop {
            let mut transfer = async_group.wait_any().unwrap();
            println!("Read: {:?} {:?}", transfer.status(), transfer.actual());
            async_group.submit(transfer).unwrap();
        }
    }
}

fn configure_endpoint<'a>(handle: &'a mut DeviceHandle, endpoint: &Endpoint) -> Result<()> {
    handle.set_active_configuration(endpoint.config)?;
    handle.claim_interface(endpoint.iface)?;
    handle.set_alternate_setting(endpoint.iface, endpoint.setting)?;
    Ok(())
}
