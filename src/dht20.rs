#![allow(dead_code)]
//! DHT20 (AHT20系) I2C 温湿度センサ簡易ドライバ
//! - 計測トリガ -> Busy解除待ち -> 7バイト取得 -> CRC -> 20bit抽出 -> 物理量
//! - CRC8: 初期値0xFF, poly=0x31
//! - 初期化時 0x71 status & 0x18 確認し不一致なら JH リセット (0x1B,0x1C,0x1E)

use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;

pub const DHT20_ADDR: u8 = 0x38;

#[derive(Debug)]
pub enum Error<E> {
    I2c(E),
    CrcMismatch,
}

pub struct Dht20<I2C, D> {
    i2c: I2C,
    delay: D,
}

impl<I2C, D, E> Dht20<I2C, D>
where
    I2C: I2c<Error = E>,
    D: DelayNs,
{
    pub fn new(i2c: I2C, delay: D) -> Self { Self { i2c, delay } }

    pub fn init(&mut self) -> Result<(), Error<E>> {
        // 電源安定待ち >=100ms
        self.delay.delay_ms(100);
        let status = self.read_status()?;
        if (status & 0x18) != 0x18 {
            self.jh_reset(0x1B)?;
            self.jh_reset(0x1C)?;
            self.jh_reset(0x1E)?;
            self.delay.delay_ms(10);
        }
        Ok(())
    }

    fn read_status(&mut self) -> Result<u8, Error<E>> {
        let mut s = [0u8; 1];
        self.i2c
            .write_read(DHT20_ADDR, &[0x71], &mut s)
            .map_err(Error::I2c)?;
        Ok(s[0])
    }

    fn jh_reset(&mut self, reg: u8) -> Result<(), Error<E>> {
        self.i2c.write(DHT20_ADDR, &[reg, 0x00, 0x00]).map_err(Error::I2c)?;
        self.delay.delay_ms(5);
        let mut regs = [0u8; 3];
        self.i2c.read(DHT20_ADDR, &mut regs).map_err(Error::I2c)?;
        self.delay.delay_ms(10);
        let wb = [0xB0 | reg, regs[1], regs[2]];
        self.i2c.write(DHT20_ADDR, &wb).map_err(Error::I2c)?;
        Ok(())
    }

    pub fn read(&mut self) -> Result<(f32, f32), Error<E>> {
        self.i2c.write(DHT20_ADDR, &[0xAC, 0x33, 0x00]).map_err(Error::I2c)?;
        // 目安 85ms 待つ (BUSY=0 になるまでループ)
        self.delay.delay_ms(85);
        loop {
            let st = self.read_status()?;
            if (st & 0x80) == 0 { break; }
            self.delay.delay_ms(2);
        }
        let mut buf = [0u8; 7];
        self.i2c.read(DHT20_ADDR, &mut buf).map_err(Error::I2c)?;
        if crc8(&buf[..6]) != buf[6] { return Err(Error::CrcMismatch); }
        let raw_h = (((buf[1] as u32) << 16) | ((buf[2] as u32) << 8) | (buf[3] as u32)) >> 4;
        let raw_t = (((buf[3] as u32) & 0x0F) << 16) | ((buf[4] as u32) << 8) | (buf[5] as u32);
        let rh = (raw_h as f32) * 100.0 / 1_048_576.0; // 2^20
        let t  = (raw_t as f32) * 200.0 / 1_048_576.0 - 50.0;
        Ok((rh, t))
    }

    pub fn delay_mut(&mut self) -> &mut D { &mut self.delay }

    pub fn free(self) -> (I2C, D) { (self.i2c, self.delay) }
}

fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0xFF;
    for &b in data {
        crc ^= b;
        for _ in 0..8 { crc = if (crc & 0x80) != 0 { (crc << 1) ^ 0x31 } else { crc << 1 }; }
    }
    crc
}
