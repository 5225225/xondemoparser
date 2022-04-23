// thanks https://github.com/TimePath/xonotic-demo-parser/blob/60ee9cf0b17644fe0b37938b1c96e6dd3ba9fc58/src/main/scala/com/timepath/xonotic/DemoParser.scala#L287

use bytes::Buf;
use std::io::Write;

struct Parser {
    buf: bytes::Bytes,
    downloaded_data: Vec<u8>,
}

trait BufExt: bytes::Buf {
    /// Reads a \n terminated string. Consumes up to and including the \n
    fn get_string(&mut self, terminator: u8) -> String {
        let mut s = String::new();

        loop {
            match self.get_u8() {
                c if c == terminator => break,
                c => s.push(char::from(c)),
            }
        }

        s
    }

    fn get_line(&mut self) -> String {
        self.get_string(10)
    }

    fn get_zstring(&mut self) -> String {
        self.get_string(0)
    }

    fn bail_dump(&mut self, err: &str) -> ! {
        let mut buf = Vec::new();

        std::io::copy(&mut self.take(256).reader(), &mut buf).unwrap();

        eprintln!("{}", nu_pretty_hex::pretty_hex(&buf));
        panic!("{}", err);
    }

    fn get_coord(&mut self) -> f32 {
        self.get_f32()
    }

    fn get_angle(&mut self) -> u16 {
        self.get_u16_le()
    }

    fn get_vector(&mut self) -> Vector {
        let x = self.get_f32();
        let y = self.get_f32();
        let z = self.get_f32();

        Vector { x, y, z }
    }
}

impl<T> BufExt for T where T: bytes::Buf {}

impl Parser {
    fn new(buf: Vec<u8>) -> Self {
        Self {
            buf: buf.into(),
            downloaded_data: Vec::new(),
        }
    }

    fn parse_header(&mut self) -> String {
        self.buf.get_line()
    }

    const CLIENT_TO_SERVER: u32 = 0x8000_0000;
    fn parse_packet(&mut self) -> Option<Packet> {
        if self.buf.remaining() == 0 {
            return None;
        }

        let s = self.buf.get_u32_le();
        let is_client_to_server = Self::CLIENT_TO_SERVER & s != 0;
        let len = s & !Self::CLIENT_TO_SERVER;

        let view_angles = self.buf.get_vector();

        let direction = if is_client_to_server {
            Direction::ClientToServer
        } else {
            Direction::ServerToClient
        };
        let buf = self.buf.copy_to_bytes(usize::try_from(len).unwrap());

        assert_eq!(direction, Direction::ServerToClient);

        Some(Packet {
            direction,
            view_angles,
            buf,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Direction {
    ClientToServer,
    ServerToClient,
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Debug)]
struct Vector {
    x: f32,
    y: f32,
    z: f32,
}

struct Packet {
    direction: Direction,
    view_angles: Vector,
    buf: bytes::Bytes,
}

#[derive(Clone, Debug)]
struct Entity {
    model_index: Option<u8>,
    frame: Option<u8>,
    colormap: Option<u8>,
    skin: Option<u8>,
    origin: Option<f32>,
    effects: Option<u8>,
    angle: Option<u16>,
    origin2: Option<f32>,
    angle2: Option<u16>,
    origin3: Option<f32>,
    angle3: Option<u16>,
    alpha: Option<u8>,
    scale: Option<u8>,
    effects2: Option<u8>,
    glowsize: Option<u8>,
    glowcolor: Option<u8>,
    colormod: Option<u8>,
    frame2: Option<u8>,
    model2: Option<u8>,
}

impl Entity {
    fn parse(mut bits: u32, from: &mut bytes::Bytes) -> Self {
        if bits & MOREBITS != 0 {
            bits |= u32::from(from.get_u8()) << 8;

            if bits & EXTEND1 != 0 {
                bits |= u32::from(from.get_u8()) << 16;

                if bits & EXTEND2 != 0 {
                    bits |= u32::from(from.get_u8()) << 24;
                }
            }
        }

        let num = if bits & LONGENTITY != 0 {
            from.get_u16_le()
        } else {
            u16::from(from.get_u8())
        };

        let model_index = (bits & MODEL != 0).then(|| from.get_u8());
        let frame = (bits & FRAME != 0).then(|| from.get_u8());
        let colormap = (bits & COLORMAP != 0).then(|| from.get_u8());
        let skin = (bits & SKIN != 0).then(|| from.get_u8());
        let effects = (bits & EFFECTS != 0).then(|| from.get_u8());
        let origin = (bits & ORIGIN1 != 0).then(|| from.get_coord());
        let angle = (bits & ANGLE1 != 0).then(|| from.get_angle());
        let origin2 = (bits & ORIGIN2 != 0).then(|| from.get_coord());
        let angle2 = (bits & ANGLE2 != 0).then(|| from.get_angle());
        let origin3 = (bits & ORIGIN3 != 0).then(|| from.get_coord());
        let angle3 = (bits & ANGLE3 != 0).then(|| from.get_angle());
        let alpha = (bits & ALPHA != 0).then(|| from.get_u8());
        let scale = (bits & SCALE != 0).then(|| from.get_u8());
        let effects2 = (bits & EFFECTS2 != 0).then(|| from.get_u8());
        let glowsize = (bits & GLOWSIZE != 0).then(|| from.get_u8());
        let glowcolor = (bits & GLOWCOLOR != 0).then(|| from.get_u8());
        let colormod = (bits & COLORMOD != 0).then(|| from.get_u8());
        let frame2 = (bits & FRAME2 != 0).then(|| from.get_u8());
        let model2 = (bits & MODEL2 != 0).then(|| from.get_u8());

        Entity {
            model_index,
            frame,
            colormap,
            skin,
            effects,
            origin,
            angle,
            origin2,
            angle2,
            origin3,
            angle3,
            alpha,
            scale,
            effects2,
            glowsize,
            glowcolor,
            colormod,
            frame2,
            model2,
        }
    }
}

impl Packet {
    /// Yields None on EOF
    fn read_command(&mut self) -> Option<Command> {
        dbg!(self.buf.remaining());
        if self.buf.remaining() == 0 {
            return None;
        }

        let cmd = self.buf.get_u8();
        if cmd == 0xff {
            // uhh the original code says -1 but this looks to be a signed number? idk.
            return None;
        }

        if cmd & 0b1000_0000 != 0 {
            return Some(Command::Entity {
                entity: Entity::parse(u32::from(cmd & 0b0111_1111), &mut self.buf),
            });
        }

        Some(match cmd {
            9 => {
                let text = self.buf.get_zstring();
                Command::StuffText { text }
                // STUFFTEXT
            }
            50 => {
                let start = self.buf.get_u32_le();
                let size = self.buf.get_u16_le();
                let mut data = vec![0u8; usize::from(size)];
                self.buf.copy_to_slice(&mut data);

                Command::DownloadData { start, data }
            }
            8 => {
                let text = self.buf.get_zstring();

                Command::Print { text }
            }
            11 => {
                let protocol = self.buf.get_u32_le();
                // assert_eq!(protocol, 3504); // PROTOCOL_DARKPLACES7
                let maxclients = self.buf.get_u8();
                let gametype = self.buf.get_u8();
                let world_message = self.buf.get_zstring();
                let mut models = Vec::new();
                let mut sounds = Vec::new();

                loop {
                    let s = self.buf.get_zstring();
                    if s.is_empty() {
                        break;
                    }
                    models.push(s);
                }

                loop {
                    let s = self.buf.get_zstring();
                    if s.is_empty() {
                        break;
                    }
                    sounds.push(s);
                }

                Command::ServerInfo {
                    protocol,
                    maxclients,
                    gametype,
                    world_message,
                    models,
                    sounds,
                }
            }
            32 => {
                let track = self.buf.get_u8();
                let does_loop = self.buf.get_u8();

                Command::CdTrack { track, does_loop }
            }
            5 => {
                let entity = self.buf.get_u16_le();
                Command::SetView { entity }
            }
            25 => {
                let i = self.buf.get_u8();
                Command::SignOnNum { i }
            }
            1 => Command::Nop,
            23 => Command::TempEntity {
                inner: parse_temp_entity(&mut self.buf),
            },
            SPAWNSTATICSOUND2 => {
                let org = self.buf.get_vector();
                let sound = self.buf.get_u16_le();
                let vol = self.buf.get_u8();
                let atten = self.buf.get_u8();

                Command::SpawnStaticSound2 {
                    org,
                    sound,
                    vol,
                    atten,
                }
            }
            _ => self.buf.bail_dump(&format!("unknown command number {cmd}")),
        })
    }
}

fn parse_temp_entity(buf: &mut bytes::Bytes) -> TempEntity {
    let ty = buf.get_u8();

    match ty {
        // Yes, I *am* assuming the id here is consistent!
        // But doing this properly requires some level of code execution of the progs.dat
        // and fuck that noise
        // so i'm going off common/net_linked.qh and server/race.qc in the xonotic pk3dir qcsrc
        86 => {
            let ty = buf.get_u8();
            dbg!(ty);
            match ty {
                11 => {
                    // RACE_NET_SERVER_RANKINGS
                    let pos = buf.get_u16_le();
                    let prev_pos = buf.get_u16_le();
                    let del = buf.get_u16_le();
                    let name = buf.get_zstring();
                    let time = u32::try_from(buf.get_uint_le(3)).unwrap();

                    TempEntity::RaceRanking {
                        pos,
                        prev_pos,
                        del,
                        name,
                        time,
                    }
                }
                8 => {
                    // RACE_NET_SERVER_RECORD
                    let time = u32::try_from(buf.get_uint_le(3)).unwrap();
                    TempEntity::ServerRecord { time }
                }
                9 => {
                    // RACE_NET_SPEED_AWARD
                    let speed = u32::try_from(buf.get_uint_le(3)).unwrap();
                    let holder = buf.get_zstring();

                    TempEntity::SpeedAward { speed, holder }
                }
                1 => {
                    // RACE_NET_CHECKPOINT_CLEAR
                    // no arguments!
                    TempEntity::RaceCheckpointClear
                }
                10 => {
                    // RACE_NET_SPEED_AWARD_BEST
                    let speed = u32::try_from(buf.get_uint_le(3)).unwrap();
                    let holder = buf.get_zstring();

                    TempEntity::BestSpeedAward { speed, holder }
                }
                15 => {
                    // RACE_NET_RANKINGS_CNT
                    let count = buf.get_u8();
                    TempEntity::RankingsCount { count }
                }
                _ => buf.bail_dump(&format!("unknown temp entity 86 type {ty}")),
            }
        }
        0x63 => {
            buf.advance(5);
            TempEntity::UnknownNintyNine
        }
        _ => buf.bail_dump(&format!("unknown tempentity type {ty:#X}")),
    }
}

#[derive(Clone, Debug)]
enum TempEntity {
    RaceRanking {
        pos: u16,
        prev_pos: u16,
        del: u16,
        name: String,
        time: u32,
    },
    RaceCheckpointClear,
    ServerRecord {
        time: u32,
    },
    SpeedAward {
        speed: u32,
        holder: String,
    },
    BestSpeedAward {
        speed: u32,
        holder: String,
    },
    RankingsCount {
        count: u8,
    },
    UnknownNintyNine,
}

#[derive(Clone, Debug)]
enum Command {
    StuffText {
        text: String,
    },

    DownloadData {
        start: u32,
        data: Vec<u8>,
    },

    Print {
        text: String,
    },

    ServerInfo {
        protocol: u32,
        maxclients: u8,
        gametype: u8,
        world_message: String,
        models: Vec<String>,
        sounds: Vec<String>,
    },

    CdTrack {
        track: u8,
        does_loop: u8,
    },

    SetView {
        entity: u16,
    },

    SignOnNum {
        i: u8,
    },
    TempEntity {
        inner: TempEntity,
    },
    Nop,
    Entity {
        entity: Entity,
    },
    SpawnStaticSound2 {
        org: Vector,
        sound: u16,
        vol: u8,
        atten: u8,
    },
}

#[derive(Clone, Debug, PartialEq, num_enum::IntoPrimitive, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum InnerCommand {
    BAD = 0,
    NOP = 1,
    DISCONNECT = 2,
    UPDATESTAT = 3,
    VERSION = 4,
    SETVIEW = 5,
    SOUND = 6,
    TIME = 7,
    PRINT = 8,
    STUFFTEXT = 9,
    SETANGLE = 10,
    SERVERINFO = 11,
    LIGHTSTYLE = 12,
    UPDATENAME = 13,
    UPDATEFRAGS = 14,
    CLIENTDATA = 15,
    STOPSOUND = 16,
    UPDATECOLORS = 17,
    PARTICLE = 18,
    DAMAGE = 19,
    SPAWNSTATIC = 20,
    SPAWNBINARY = 21,
    SPAWNBASELINE = 22,
    TEMP_ENTITY = 23,
    SETPAUSE = 24,
    SIGNONNUM = 25,
    CENTERPRINT = 26,
    KILLEDMONSTER = 27,
    FOUNDSECRET = 28,
    SPAWNSTATICSOUND = 29,
    INTERMISSION = 30,
    FINALE = 31,
    CDTRACK = 32,
    SELLSCREEN = 33,
    CUTSCENE = 34,
    SHOWLMP = 35,
    HIDELMP = 36,
    SKYBOX = 37,
    DOWNLOADDATA = 50,
    UPDATESTATUBYTE = 51,
    EFFECT = 52,
    EFFECT2 = 53,
    Sound2OrPrecache = 54,
    SPAWNBASELINE2 = 55,
    SPAWNSTATIC2 = 56,
    ENTITIES = 57,
    CSQCENTITIES = 58,
    SPAWNSTATICSOUND2 = 59,
    TRAILPARTICLES = 60,
    POINTPARTICLES = 61,
    POINTPARTICLES1 = 62,
}

const TE_SPIKE: u8 = 0;
const TE_SUPERSPIKE: u8 = 1;
const TE_GUNSHOT: u8 = 2;
const TE_EXPLOSION: u8 = 3;
const TE_TAREXPLOSION: u8 = 4;
const TE_LIGHTNING1: u8 = 5;
const TE_LIGHTNING2: u8 = 6;
const TE_WIZSPIKE: u8 = 7;
const TE_KNIGHTSPIKE: u8 = 8;
const TE_LIGHTNING3: u8 = 9;
const TE_LAVASPLASH: u8 = 10;
const TE_TELEPORT: u8 = 11;
const TE_EXPLOSION2: u8 = 12;
const TE_EXPLOSIONRGB: u8 = 53;
const TE_GUNSHOTQUAD: u8 = 57;
const TE_EXPLOSIONQUAD: u8 = 70;
const TE_SPIKEQUAD: u8 = 58;
const TE_SUPERSPIKEQUAD: u8 = 59;

const MOREBITS: u32 = 1 << 0;
const ORIGIN1: u32 = 1 << 1;
const ORIGIN2: u32 = 1 << 2;
const ORIGIN3: u32 = 1 << 3;
const ANGLE2: u32 = 1 << 4;
const STEP: u32 = 1 << 5;
const FRAME: u32 = 1 << 6;
const SIGNAL: u32 = 1 << 7;
const ANGLE1: u32 = 1 << 8;
const ANGLE3: u32 = 1 << 9;
const MODEL: u32 = 1 << 10;
const COLORMAP: u32 = 1 << 11;
const SKIN: u32 = 1 << 12;
const EFFECTS: u32 = 1 << 13;
const LONGENTITY: u32 = 1 << 14;
const EXTEND1: u32 = 1 << 15;
const DELTA: u32 = 1 << 16;
const ALPHA: u32 = 1 << 17;
const SCALE: u32 = 1 << 18;
const EFFECTS2: u32 = 1 << 19;
const GLOWSIZE: u32 = 1 << 20;
const GLOWCOLOR: u32 = 1 << 21;
const COLORMOD: u32 = 1 << 22;
const EXTEND2: u32 = 1 << 23;
const GLOWTRAIL: u32 = 1 << 24;
const VIEWMODEL: u32 = 1 << 25;
const FRAME2: u32 = 1 << 26;
const MODEL2: u32 = 1 << 27;
const EXTERIORMODEL: u32 = 1 << 28;
const UNUSED29: u32 = 1 << 29;
const UNUSED30: u32 = 1 << 30;
const EXTEND3: u32 = 1 << 31;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let d = include_bytes!("../demos/2022-04-09_03-30_kool_red2.dem");
        let d = include_bytes!("../demos/2022-04-01_00-33_sl1k-kleo.dem");
        let mut p = Parser::new(d.to_vec());
        let h = p.parse_header();
        assert_eq!(h, "-1");

        while let Some(mut packet) = p.parse_packet() {
            while let Some(cmd) = packet.read_command() {
                dbg!(&cmd);
            }
        }
    }

    #[test]
    #[ignore = "doesn't work yet"]
    fn little_bot_orchestra() {
        // let d = include_bytes!("../demos/2022-04-09_03-30_kool_red2.dem");
        let d = include_bytes!("../demos/little-bot-orchestra.dem");
        let mut p = Parser::new(d.to_vec());
        let h = p.parse_header();
        assert_eq!(h, "-1");

        while let Some(mut packet) = p.parse_packet() {
            while let Some(cmd) = packet.read_command() {
                match cmd {
                    Command::DownloadData { start, data } => {
                        p.downloaded_data.extend(data);
                    }
                    Command::StuffText { text } if text.contains("cl_downloadfinished") => {
                        println!("Dumping downloaded data to prog.dat");
                        std::fs::File::create("/home/jess/src/xondemoparser/prog.dat")
                            .unwrap()
                            .write_all(&p.downloaded_data)
                            .unwrap();
                    }
                    other => drop(dbg!(&other)),
                }
            }
        }
    }
}
