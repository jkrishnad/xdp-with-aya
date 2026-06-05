#![no_std]
#![no_main]

use core::mem;

use aya_ebpf::{bindings::xdp_action, macros::xdp, programs::XdpContext};
use aya_log_ebpf::info;
use network_types::{
    eth::{EthHdr, EtherType},
    ip::{IpError, IpProto, Ipv4Hdr},
    tcp::TcpHdr,
    udp::UdpHdr,
};

#[xdp]
pub fn xdp_filter(ctx: XdpContext) -> u32 {
    match try_xdp_filter(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_ABORTED,
    }
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, u32> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(xdp_action::XDP_ABORTED);
    }

    Ok((start + offset) as *const T)
}

fn try_xdp_filter(ctx: XdpContext) -> Result<u32, u32> {
    info!(&ctx, "received a packet");
    // Reciving the Ethernet headers
    let ethhdr: *const EthHdr = ptr_at::<EthHdr>(&ctx, 0)?;

    // Verifying the Ethernet type is IPv4
    match unsafe { (*ethhdr).ether_type() } {
        Ok(EtherType::Ipv4) => {}
        _ => return Err(xdp_action::XDP_ABORTED),
    }

    // Verifying the IP protocol is IPv4
    let ipv4hdr: *const Ipv4Hdr = ptr_at::<Ipv4Hdr>(&ctx, EthHdr::LEN)?;

    // Source address from where the packet was sent
    let source_addr = u32::from_be_bytes(unsafe { (*ipv4hdr).src_addr });
    info!(&ctx, "source_addr: {}", source_addr);

    // Next protocol box
    let proto = unsafe { (*ipv4hdr).proto() }
        .map_err(|IpError::InvalidProto(_proto)| xdp_action::XDP_ABORTED)?;

    // Source port from where the packet was sent
    let source_port = match proto {
        IpProto::Tcp => {
            let tcp_hdr: *const TcpHdr = ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            u16::from_be_bytes(unsafe { (*tcp_hdr).source })
        }
        IpProto::Udp => {
            let upd_hdr: *const UdpHdr = ptr_at(&ctx, EthHdr::LEN + Ipv4Hdr::LEN)?;
            u16::from_be_bytes(unsafe { (*upd_hdr).src_port().to_le_bytes() })
        }
        _ => return Err(xdp_action::XDP_ABORTED),
    };

    info!(&ctx, "SRC IP: {:i}, SRC PORT: {}", source_addr, source_port);
    Ok(xdp_action::XDP_PASS)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
