// NOTE: my trackpad is doxing me.
//
//   ...
//
//   i was trying to figure out how to rename it using bluetoothctl and other random bluetooth
//   related tools that i could find online; but no luck with that.
//
//   i was finding ways to change alias, but not the actual name.
//
//   the most useful thing that i discovered related to bluetooth was
//   $ busctl tree org.bluez
//   and then
//   $ busctl introspect org.bluez /org/bluez/hci0/dev_XX
//   there's .Alias (the thing i was getting a lot of search results on (how to change)).
//   but also there's .Name field - and that seems like something of interest.
//
//   ...
//
//   at one point i realized there's this thing called hid.
//
//   hid = human interface device.
//   see https://docs.kernel.org/hid/hidintro.html
//
//   in rust there's hidapi crate; so i started using it.
//   at this point i didn't have any cli util capable of providing any kind of info about hids.
//
//   first thing i did - tried to dump device list (from hidapi crate).
//   none has named anything apple nor trackpad.
//   some names are empty.
//   vendor id and product id fields are interesting.
//   so i started searching online for apple vendor id and trackpad product id.
//   :HidIds
//
//   i started digging more into what hidapi can do - get_feature_report function was interesting,
//   but i coudn't figure out how to use it right away hehe.
//   turned out you need to fetch report first!
//
//   with that i finally was able to fetch feature bytes.
//
//   i tried to use hid-feature crate under https://github.com/hidutils to decode and dump some info
//   - thing didn't work at all.
//   unexpected eof or something bla bla ..
//   i don't remember (i am writing this after i have figured out how to read it).
//
//   somewhere along the way i discovered hid-tools package
//   $ hid-decode /sys/class/hidraw/hidraw9/device/report_descriptor
//   was good!
//   it helped me get some glance at what reports actually contain.
//   the output didn't contain direct answers, but contained some good clues!
//
//   i started trying to dump variables (hidparser::ReportField::Variable of feature) as strings
//   (because name is a string, right?? (yes!))
//   and that was it.
//   :NameReportId
//
//   i cleand up shit and composed print_feature_report.
//   note that get_feature_report returns length equal to to the actual length of name, but
//   report's fields reach into bytes that are out of bounds of that - full range of
//   report.size_in_bits / 8.
//   :NameMaxLen
//
//   somewhat irrelevant to the cause, but worth noting that bits in features are byte-aligned, but
//   not in input reports and not in output reports.
//
//   ...
//
//   now i need to figure out how to change it.
//
//   okay, that was omega ez!
//   send_feature_report with id and name bytes.
//

// NOTE: googled "apple hid vendor id"
//   - found https://discussions.apple.com/thread/256213389
//   - ended up on https://theapplewiki.com/wiki/Bluetooth_PIDs
//   - searched for 0x004 in linux kernel, found
//     https://github.com/torvalds/linux/blob/f0100363d8c374bd8e9ea7c9ba02744f0b802ca4/drivers/hid/hid-ids.h#L97
//   :HidIds
const BT_VENDOR_ID_APPLE: u16 = 0x004c;
const USB_DEVICE_ID_APPLE_MAGICTRACKPAD2: u16 = 0x0265;

// NOTE: this was discovered by dumping stuff; see :NameReportId
const MAGICTRACKPAD2_NAME_FEATURE_REPORT_ID: u8 = 85;
// NOTE: see :NameMaxLen
const MAGICTRACKPAD2_NAME_MAX_LEN: usize = 64;

type Result<T> = std::result::Result<T, std::boxed::Box<dyn std::error::Error>>;

#[rustfmt::skip]
fn print_device_info(dev_info: &hidapi::DeviceInfo) {
    println!("{:.<19} = {:?}", "path", dev_info.path());
    println!("{:.<19} = x0{:.<4x} bt apple? {}", "vendor_id", dev_info.vendor_id(), dev_info.vendor_id() == BT_VENDOR_ID_APPLE);
    println!("{:.<19} = x0{:.<4x} usb apple magictrackpad2? {}", "product_id", dev_info.product_id(), dev_info.product_id() == USB_DEVICE_ID_APPLE_MAGICTRACKPAD2);
    println!("{:.<19} = {:?}", "serial_number", dev_info.serial_number());
    println!("{:.<19} = {}", "release_number", dev_info.release_number());
    println!("{:.<19} = {:?}", "manufacturer_string", dev_info.manufacturer_string());
    println!("{:.<19} = {:?}", "product_string", dev_info.product_string());
    println!("{:.<19} = 0x{:x}", "usage_page", dev_info.usage_page());
    println!("{:.<19} = 0x{:x}", "usage", dev_info.usage());
    println!("{:.<19} = {}", "interface_number", dev_info.interface_number());
    println!("{:.<19} = {:?}", "bus_type", dev_info.bus_type());
}

fn print_feature_report(rep: &hidparser::Report, data: &[u8]) -> Result<()> {
    // NOTE: skip id.
    let data = &data[1..];
    for (field_index, field) in rep.fields.iter().enumerate() {
        match field {
            hidparser::ReportField::Variable(var) => {
                print!("{field_index:<3} ");
                // print!("{} ", var.bits.start % 8 == 0);
                // print!("{} ", var.bits.end % 8 == 0);
                let bytes = &data[var.bits.start as usize / 8..var.bits.end as usize / 8];
                let ctr = str::from_utf8(bytes).unwrap_or("<invalid>");
                print!("\"{ctr}\"");
                print!("\n");
            }
            _ => {}
        }
    }
    Ok(())
}

fn rename_magictrackpad2(dev: &hidapi::HidDevice, name: &str) -> Result<()> {
    assert!(name.len() <= MAGICTRACKPAD2_NAME_MAX_LEN);

    // NOTE: + 1 for the report id.
    let mut buf = [0u8; MAGICTRACKPAD2_NAME_MAX_LEN + 1];
    buf[0] = MAGICTRACKPAD2_NAME_FEATURE_REPORT_ID;
    buf[1..1 + name.len()].copy_from_slice(name.as_bytes());

    dev.send_feature_report(&buf)?;
    Ok(())
}

fn main() -> Result<()> {
    let api = hidapi::HidApi::new()?;

    let dev_info = api
        .device_list()
        .find(|it| {
            it.vendor_id() == BT_VENDOR_ID_APPLE
                && it.product_id() == USB_DEVICE_ID_APPLE_MAGICTRACKPAD2
        })
        .expect("could not find device");
    print_device_info(dev_info);

    let dev = api.open(dev_info.vendor_id(), dev_info.product_id())?;

    let mut report_desc_buf = [0u8; 1 << 10];
    let report_desc_len = dev.get_report_descriptor(&mut report_desc_buf)?;
    assert!(report_desc_len <= report_desc_buf.len());
    let report_desc = hidparser::parse_report_descriptor(&report_desc_buf[..report_desc_len])
        .expect("could not parse report descriptor");

    let name_feature_rep = {
        let name_feature_report_id = hidparser::report_data_types::ReportId::from(
            MAGICTRACKPAD2_NAME_FEATURE_REPORT_ID as u32,
        );
        report_desc
            .features
            .iter()
            .find(|it| it.report_id == Some(name_feature_report_id))
            .expect("could not find name feature report")
    };

    let name_feature_rep_id = {
        let rep_id = name_feature_rep.report_id.expect("report id");
        //
        //   why are you hiding values from by wrapping them into newtypes??
        //
        //   why are you doing u32?
        //
        //   QUOTE:
        //     > Whenever a Report ID is needed it is transmitted as the first byte of any report.
        //     - https://docs.kernel.org/hid/hidintro.html
        let rep_id: u32 = rep_id.into();
        let rep_id: u8 = rep_id.try_into()?;
        rep_id
    };

    // NOTE: this was discovered. see :NameMaxLen
    assert!(name_feature_rep.size_in_bits % 8 == 0);
    assert!(name_feature_rep.size_in_bits / 8 == MAGICTRACKPAD2_NAME_MAX_LEN);

    let mut name_feature_rep_buf = [0u8; 1 + MAGICTRACKPAD2_NAME_MAX_LEN];
    // QUOTE:
    //   > Set the first byte of buf to the ‘Report ID’ of the report to be read. Upon return, the
    //   first byte will still contain the Report ID, and the report data will start in buf[1]
    //   - doc comment of get_feature_report.
    name_feature_rep_buf[0] = name_feature_rep_id;
    let name_feature_rep_len = dev.get_feature_report(&mut name_feature_rep_buf)?;

    print_feature_report(name_feature_rep, &name_feature_rep_buf)?;

    let old_name =
        std::string::String::from_utf8_lossy(&name_feature_rep_buf[1..name_feature_rep_len]);
    // NOTE: might have put this into cli args, but here i am done.
    let new_name = "hi";
    rename_magictrackpad2(&dev, new_name)?;
    println!("renamed \"{old_name}\" to \"{new_name}\"");

    Ok(())
}
