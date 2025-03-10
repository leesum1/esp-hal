//! Interrupt Preemption
//!
//! An example of how an interrupt can be preempted by another with higher
//! priority. Should show higher-numbered software interrupts happening during
//! the handling of lower-numbered ones.

#![no_std]
#![no_main]

use core::cell::RefCell;

use critical_section::Mutex;
use esp32c6_hal::{
    clock::ClockControl,
    interrupt::{self},
    peripherals::{self, Peripherals},
    prelude::*,
    riscv,
    system::{SoftwareInterrupt, SoftwareInterruptControl},
    timer::TimerGroup,
    Rtc,
};
use esp_backtrace as _;

static SWINT: Mutex<RefCell<Option<SoftwareInterruptControl>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let mut system = peripherals.PCR.split();
    let clockctrl = system.clock_control;
    let sw_int = system.software_interrupt_control;
    let clocks = ClockControl::boot_defaults(clockctrl).freeze();

    // Disable the watchdog timers. For the ESP32-C3, this includes the Super WDT,
    // the RTC WDT, and the TIMG WDTs.
    let mut rtc = Rtc::new(peripherals.LP_CLKRST);
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt1 = timer_group1.wdt;

    rtc.swd.disable();
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    critical_section::with(|cs| SWINT.borrow_ref_mut(cs).replace(sw_int));

    interrupt::enable(
        peripherals::Interrupt::FROM_CPU_INTR0,
        interrupt::Priority::Priority1,
    )
    .unwrap();
    interrupt::enable(
        peripherals::Interrupt::FROM_CPU_INTR1,
        interrupt::Priority::Priority2,
    )
    .unwrap();
    interrupt::enable(
        peripherals::Interrupt::FROM_CPU_INTR2,
        interrupt::Priority::Priority2,
    )
    .unwrap();
    interrupt::enable(
        peripherals::Interrupt::FROM_CPU_INTR3,
        interrupt::Priority::Priority15,
    )
    .unwrap();
    unsafe { riscv::interrupt::enable() }

    // raise mid priority interrupt.
    // The handler raises one interrupt at lower priority, one at same and one at
    // higher. We expect to see the higher priority served immeiately before
    // exiting the handler Once the handler is exited we expect to see same
    // priority and low priority interrupts served in that order
    critical_section::with(|cs| {
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .raise(SoftwareInterrupt::SoftwareInterrupt1);
    });
    loop {}
}

#[interrupt]
fn FROM_CPU_INTR0() {
    esp_println::println!("SW interrupt0");
    critical_section::with(|cs| {
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .reset(SoftwareInterrupt::SoftwareInterrupt0);
    });
}
#[interrupt]
fn FROM_CPU_INTR1() {
    esp_println::println!("SW interrupt1 entry");
    critical_section::with(|cs| {
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .reset(SoftwareInterrupt::SoftwareInterrupt1);
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .raise(SoftwareInterrupt::SoftwareInterrupt2); // raise interrupt at same priority
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .raise(SoftwareInterrupt::SoftwareInterrupt3); // raise interrupt at higher priority
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .raise(SoftwareInterrupt::SoftwareInterrupt0); // raise interrupt at
                                                           // lower priority
    });
    esp_println::println!("SW interrupt1 exit");
}
#[interrupt]
fn FROM_CPU_INTR2() {
    esp_println::println!("SW interrupt2");
    critical_section::with(|cs| {
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .reset(SoftwareInterrupt::SoftwareInterrupt2);
    });
}
#[interrupt]
fn FROM_CPU_INTR3() {
    esp_println::println!("SW interrupt3");
    critical_section::with(|cs| {
        SWINT
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .reset(SoftwareInterrupt::SoftwareInterrupt3);
    });
}
