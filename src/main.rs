#![no_std]
#![no_main]

use rp_pico::hal;
use hal::pac;
use panic_halt as _;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

// UART で送信するためのユーティリティ: 0-100 を 10 進 ASCII に変換
fn u8_to_decimal_buf(n: u8, buf: &mut [u8; 4]) -> &[u8] {
    // 最大 "100" + 終端不要 (戻り値で長さ制御)
    let mut i = 0;
    if n == 100 { // 特別ケース
        buf[0] = b'1';
        buf[1] = b'0';
        buf[2] = b'0';
        return &buf[..3];
    }
    let tens = n / 10;
    let ones = n % 10;
    if tens > 0 { // 2 桁
        buf[i] = b'0' + tens;
        i += 1;
    }
    buf[i] = b'0' + ones;
    i += 1;
    &buf[..i]
}

#[hal::entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut timer = hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let sio = hal::Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // LED (オンボード) 確認用
    let mut led = pins.gpio25.into_push_pull_output();
    let _ = led.set_high();

    // UART0 の標準ピン: TX=GPIO0, RX=GPIO1
    // 送信機なので RX は未接続でもよい (配線してもしなくても OK)
    let uart_pins = (
        pins.gpio0.into_function::<hal::gpio::FunctionUart>(), // TX
        pins.gpio1.into_function::<hal::gpio::FunctionUart>(), // RX (未使用)
    );

    use hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
    use hal::fugit::RateExtU32; // Hz() extension re-export
    use hal::clocks::Clock; // freq() trait
    let uart_config = UartConfig::new(115_200.Hz(), DataBits::Eight, None, StopBits::One);

    let uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(uart_config, clocks.peripheral_clock.freq())
        .unwrap();

    let mut count: u8 = 0;
    let mut buf = [0u8; 4];
    loop {
        // LED をトグルして送信タイミング確認
        let _ = led.set_low();
        timer.delay_ms(100);
        let _ = led.set_high();

        let slice = u8_to_decimal_buf(count, &mut buf);
    uart.write_full_blocking(slice);
    uart.write_full_blocking(b"\r\n"); // 行終端 (CRLF)

        // 次の値
        count = if count == 100 { 0 } else { count + 1 };
        timer.delay_ms(400); // 合計 ~500ms 間隔
    }
}