// ============================================================
// T-Beam V1.2 — Oyster field receiver
// Receives 9-byte WindPacket from the Pico, shows RSSI/SNR on
// the OLED, logs every packet + GPS position to flash (CSV).
//
// DUMP MODE: hold the user button (GPIO38) while pressing RST.
// The log prints over USB serial instead of receiving.
// ============================================================

#include <Wire.h>
#include <SPI.h>
#include <RadioLib.h>
#include <XPowersLib.h>
#include <TinyGPS++.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>
#include <LittleFS.h>

// ---- T-Beam V1.2 pins ----
#define LORA_CS   18
#define LORA_IRQ  26   // DIO0
#define LORA_RST  23
#define LORA_DIO1 33
#define LORA_SCK   5
#define LORA_MISO 19
#define LORA_MOSI 27
#define I2C_SDA   21
#define I2C_SCL   22
#define GPS_RX    34
#define GPS_TX    12
#define BUTTON    38   // user button; if dump mode never triggers, try 0

XPowersAXP2101 PMU;
Adafruit_SSD1306 display(128, 64, &Wire, -1);
TinyGPSPlus gps;
HardwareSerial gpsSerial(1);
SX1276 radio = new Module(LORA_CS, LORA_IRQ, LORA_RST, LORA_DIO1);

volatile bool rxFlag = false;
void setRxFlag() { rxFlag = true; }   // called by interrupt on DIO0

File logFile;

uint16_t lastSeq = 0;
float lastWind = 0, lastBatt = 0, lastRssi = 0, lastSnr = 0;
uint8_t  lastVersion = 0;              // 1 or 2: which format this packet used
float    lastGust = 0, lastLull = 0;   // v2 only - meaningless until lastVersion == 2
uint16_t lastWindowS = 0;              // v2 only: reporting window, in seconds
unsigned long lastRxMs = 0;
uint32_t rxCount = 0;

uint8_t crc8_sum(const uint8_t* d, size_t n) {
  uint8_t c = 0;
  for (size_t i = 0; i < n; i++) c = (uint8_t)(c + d[i]);
  return c;
}

// v1 (9 B) and v2 (18 B) BOTH start with 0xAA, so dispatch on LENGTH,
// never on the version byte - byte 1 is not trustworthy until the CRC
// has passed. Returns true and fills the last* variables only if good.
bool parsePacket(const uint8_t* b, size_t len) {
  size_t crcAt;
  if (len == 9)       crcAt = 8;    // v1: sum of bytes 0..7,  crc at index 8
  else if (len == 18) crcAt = 17;   // v2: sum of bytes 0..16, crc at index 17
  else return false;                // unknown length -> drop it (fail closed)

  if (b[0] != 0xAA) return false;
  if (crc8_sum(b, crcAt) != b[crcAt]) return false;

  if (len == 9) {                                 // ---- v1 ----
    lastSeq     = (b[6] << 8) | b[7];
    lastWind    = ((b[2] << 8) | b[3]) / 10.0;    // the mean
    lastBatt    = (b[4] << 8) | b[5];
    lastVersion = 1;
  } else {                                        // ---- v2 ----
    lastSeq     = (b[12] << 8) | b[13];
    lastWind    = ((b[4] << 8) | b[5]) / 10.0;    // avg - same 0.1 m/s scale
    lastGust    = ((b[6] << 8) | b[7]) / 10.0;
    lastLull    = ((b[8] << 8) | b[9]) / 10.0;
    lastBatt    = (b[10] << 8) | b[11];
    lastWindowS = (b[14] << 8) | b[15];
    lastVersion = 2;
  }
  return true;
}

void setup() {
  Serial.begin(115200);
  delay(500);
  pinMode(BUTTON, INPUT_PULLUP);

  Wire.begin(I2C_SDA, I2C_SCL);

  // Power rails: ALDO2 feeds the SX1276, ALDO3 feeds the GPS
  PMU.begin(Wire, AXP2101_SLAVE_ADDRESS, I2C_SDA, I2C_SCL);
  PMU.setALDO2Voltage(3300); PMU.enableALDO2();
  PMU.setALDO3Voltage(3300); PMU.enableALDO3();

  display.begin(SSD1306_SWITCHCAPVCC, 0x3C);
  display.clearDisplay();
  display.setTextSize(1);
  display.setTextColor(SSD1306_WHITE);
  display.setCursor(0, 0);
  display.println("OYSTER FIELD RX");
  display.display();

  LittleFS.begin(true);

  // Dump mode: button held at boot -> print log over serial, then stop
  if (digitalRead(BUTTON) == LOW) {
    File f = LittleFS.open("/log.csv", "r");
    if (f) {
      Serial.println("---- BEGIN LOG ----");
      while (f.available()) Serial.write(f.read());
      Serial.println("---- END LOG ----");
      f.close();
    } else {
      Serial.println("no log file");
    }
    display.println("DUMP MODE");
    display.println("see serial monitor");
    display.display();
    while (true) delay(1000);
  }

  bool newFile = !LittleFS.exists("/log.csv");
  logFile = LittleFS.open("/log.csv", "a");
  if (newFile) logFile.println("utc,lat,lon,sats,seq,wind_ms,batt_mv,rssi,snr");

  gpsSerial.begin(9600, SERIAL_8N1, GPS_RX, GPS_TX);

  SPI.begin(LORA_SCK, LORA_MISO, LORA_MOSI, LORA_CS);
  int state = radio.begin(868.0, 125.0, 9, 5, 0x12, 10, 8, 0);
  if (state != RADIOLIB_ERR_NONE) {
    display.print("radio fail ");
    display.println(state);
    display.display();
    while (true) delay(1000);
  }
  radio.setDio0Action(setRxFlag, RISING);
  radio.startReceive();
}

void loop() {
  // hold the user button 3 s -> power off (battery stays in)
  static unsigned long btnDown = 0;
  if (digitalRead(BUTTON) == LOW) {
    if (btnDown == 0) btnDown = millis();
    if (millis() - btnDown > 3000) {
      display.clearDisplay();
      display.setCursor(0, 0);
      display.println("powering off...");
      display.display();
      logFile.flush();
      logFile.close();      // seal the log properly
      delay(500);
      PMU.shutdown();       // AXP2101 cuts all rails
    }
  } else {
  btnDown = 0;
  }
  // feed the GPS parser constantly
  while (gpsSerial.available()) gps.encode(gpsSerial.read());
  if (rxFlag) {
    rxFlag = false;

    uint8_t buf[64];
    size_t len = radio.getPacketLength();
    if (len > sizeof(buf)) len = sizeof(buf);
    int state = radio.readData(buf, len);

      if (state == RADIOLIB_ERR_NONE && parsePacket(buf, len)) {


      lastRssi = radio.getRSSI();
      lastSnr  = radio.getSNR();
      lastRxMs = millis();
      rxCount++;

      char ts[12] = "no-fix";
      if (gps.time.isValid()) {
        snprintf(ts, sizeof(ts), "%02d:%02d:%02d",
                 gps.time.hour(), gps.time.minute(), gps.time.second());
      }
      double lat = gps.location.isValid() ? gps.location.lat() : 0.0;
      double lon = gps.location.isValid() ? gps.location.lng() : 0.0;

      String line = String(ts) + "," + String(lat, 5) + "," + String(lon, 5) + "," +
                    String(gps.satellites.value()) + "," + String(lastSeq) + "," +
                    String(lastWind, 1) + "," + String((int)lastBatt) + "," +
                    String(lastRssi, 1) + "," + String(lastSnr, 1);
      logFile.println(line);
      logFile.flush();          // survive sudden power-off
      Serial.println(line);     // USB backup / live monitoring
    }
    radio.startReceive();       // re-arm for the next packet
  }

  updateDisplay();
}

void updateDisplay() {
  static unsigned long last = 0;
  if (millis() - last < 500) return;   // refresh twice per second
  last = millis();

  display.clearDisplay();
  display.setCursor(0, 0);
  display.println("OYSTER FIELD RX");
  if (rxCount == 0) {
    display.println("waiting...");
  } else {
    display.print("seq ");   display.print(lastSeq);
    display.print(" age ");  display.print((millis() - lastRxMs) / 1000); display.println("s");
    display.print("RSSI ");  display.print(lastRssi, 0); display.println(" dBm");
    display.print("SNR  ");  display.print(lastSnr, 1);  display.println(" dB");
  }
  display.print("sats "); display.println(gps.satellites.value());
  if (gps.location.isValid()) {
    display.print(gps.location.lat(), 4);
    display.print(",");
    display.println(gps.location.lng(), 4);
  }
  display.print("batt "); display.print(PMU.getBattVoltage()); display.println(" mV");
  display.display();
}
