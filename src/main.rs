#![no_std]
#![no_main]

use rp_pico::hal;
use hal::pac;
use panic_halt as _;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

mod dht20; // DHT20 ドライバ

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

    let timer = hal::timer::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
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

    let mut count: u8 = 0;               // UARTで送るカウンタ
    let mut buf = [0u8; 4];              // カウンタ表示用バッファ
    let mut loop_ticks: u8 = 0;          // 500ms 単位カウンタ (6で3秒)
    let mut last_rh: f32 = 0.0;          // 最新湿度（デバッグ確認用）
    let mut last_temp: f32 = 0.0;        // 最新温度（デバッグ確認用）

    // ==== DHT20 I2C 初期化 (I2C0 SDA=GP4, SCL=GP5) ====
    use hal::gpio::FunctionI2C;
    let sda = pins.gpio4.into_function::<FunctionI2C>();
    let scl = pins.gpio5.into_function::<FunctionI2C>();
    // RateExtU32 は既に UART 設定時にインポート済み
    let i2c = hal::i2c::I2C::new_controller(
        pac.I2C0,
        sda,
        scl,
        100_000u32.Hz(), // 100kHz
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );

    let mut dht = dht20::Dht20::new(i2c, timer);
    
    // 初期化結果をチェック＆デバッグ出力
    match dht.init() {
        Ok(_) => {
            uart.write_full_blocking(b"DHT20 init SUCCESS\r\n");
        }
        Err(_) => {
            uart.write_full_blocking(b"DHT20 init FAILED - check wiring!\r\n");
        }
    }

    loop {
        // LED トグル (送信直前 100ms 消灯)
        let _ = led.set_low();
        dht.delay_mut().delay_ms(100);
        let _ = led.set_high();

        // UART 送信 (0-100)
        let slice = u8_to_decimal_buf(count, &mut buf);
        uart.write_full_blocking(slice);
        uart.write_full_blocking(b"\r\n");
        count = if count == 100 { 0 } else { count + 1 };

        // 残り 400ms 待ち (合計 ~500ms 周期)
        dht.delay_mut().delay_ms(400);
        loop_ticks = loop_ticks.wrapping_add(1); // 0..=255

        // 3秒ごと（6回に1回）に温湿度を読み取り
        if loop_ticks % 6 == 0 {
            match dht.read() {
                Ok((rh, t)) => {
                    last_rh = rh;
                    last_temp = t;
                    
                    // デバッグ出力（簡易版）
                    uart.write_full_blocking(b"RH=");
                    // 湿度を整数で表示（簡易）
                    let rh_int = rh as u32;
                    if rh_int >= 100 {
                        uart.write_full_blocking(b"99+");
                    } else {
                        let rh_str = u8_to_decimal_buf(rh_int as u8, &mut buf);
                        uart.write_full_blocking(rh_str);
                    }
                    uart.write_full_blocking(b"%  T=");
                    
                    // 温度を整数で表示（簡易）
                    let t_int = t as i32;
                    if t_int < 0 {
                        uart.write_full_blocking(b"-");
                        let t_abs = (-t_int) as u8;
                        let t_str = u8_to_decimal_buf(t_abs, &mut buf);
                        uart.write_full_blocking(t_str);
                    } else if t_int >= 100 {
                        uart.write_full_blocking(b"99+");
                    } else {
                        let t_str = u8_to_decimal_buf(t_int as u8, &mut buf);
                        uart.write_full_blocking(t_str);
                    }
                    uart.write_full_blocking(b"C\r\n");
                }
                Err(_) => {
                    uart.write_full_blocking(b"DHT20 read ERROR - sensor not responding\r\n");
                }
            }
        }
    }
}