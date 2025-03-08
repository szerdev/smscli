use core::fmt;
use std::{error::Error, str::FromStr};

use futures::{SinkExt, StreamExt};
use rusmpp::{
    codec::command_codec::CommandCodec,
    commands::{
        command::Command,
        pdu::Pdu,
        types::{
            command_id::CommandId, command_status::CommandStatus, EsmClass, InterfaceVersion, Npi,
            RegisteredDelivery, ServiceType, Ton,
        },
    },
    pdu::{Bind, SubmitSm},
    types::{COctetString, OctetString},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Mutex,
};
use tokio_util::codec::{FramedRead, FramedWrite};

pub struct SmppConnection<R, W> {
    reader: Mutex<FramedRead<R, CommandCodec>>,
    writer: Mutex<FramedWrite<W, CommandCodec>>,
    connected: bool,
}

#[derive(Debug)]
pub enum SmppClientError {
    NotConnected,
    AlreadyConnected,
    SubmitFailed(CommandStatus),
    BindFailed(CommandStatus),
    UnbindFailed,
}

impl fmt::Display for SmppClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SmppClientError::NotConnected => write!(f, "Not connected"),
            SmppClientError::AlreadyConnected => write!(f, "Already connected"),
            SmppClientError::SubmitFailed(status) => write!(f, "Submit failed: {:?}", status),
            SmppClientError::BindFailed(status) => {
                write!(f, "Binding to server failed: {:?}", status)
            }
            SmppClientError::UnbindFailed => write!(f, "Unbind failed, but nobody cares."),
        }
    }
}

impl Error for SmppClientError {}

impl<R, W> SmppConnection<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Mutex::new(FramedRead::new(reader, CommandCodec {})),
            writer: Mutex::new(FramedWrite::new(writer, CommandCodec {})),
            connected: false,
        }
    }

    pub async fn bind_transceiver(
        &mut self,
        system_id: &str,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.connected {
            return Err(Box::new(SmppClientError::AlreadyConnected));
        }

        let command = Command::new(
            CommandStatus::EsmeRok,
            1,
            Bind::builder()
                .system_id(COctetString::from_str(system_id)?)
                .password(COctetString::from_str(password)?)
                .system_type(COctetString::empty())
                .interface_version(InterfaceVersion::Smpp3_4)
                .addr_ton(Ton::Unknown)
                .addr_npi(Npi::Unknown)
                .address_range(COctetString::empty())
                .build()
                .into_bind_transceiver(),
        );

        let mut r = self.reader.lock().await;
        let mut w = self.writer.lock().await;
        w.send(&command).await?;

        while let Some(Ok(command)) = r.next().await {
            if let Some(Pdu::BindTransceiverResp(_)) = command.pdu() {
                if let CommandStatus::EsmeRok = command.command_status {
                    self.connected = true;
                    break;
                } else {
                    return Err(Box::new(SmppClientError::BindFailed(
                        command.command_status,
                    )));
                }
            }
        }

        Ok(())
    }

    pub async fn submit_sm(
        &mut self,
        msisdn: &str,
        text: &str,
        source_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.connected {
            return Err(Box::new(SmppClientError::NotConnected));
        }

        let ton = if source_addr.chars().all(|c| c.is_ascii_digit()) {
            Ton::Unknown
        } else {
            Ton::Alphanumeric
        };

        let command = Command::new(
            CommandStatus::EsmeRok,
            2,
            SubmitSm::builder()
                .serivce_type(ServiceType::default())
                .source_addr_ton(ton)
                .source_addr_npi(Npi::Unknown)
                .source_addr(COctetString::from_str(source_addr)?)
                .destination_addr(COctetString::from_str(msisdn)?)
                .dest_addr_ton(Ton::International)
                .dest_addr_npi(Npi::Isdn)
                .esm_class(EsmClass::default())
                .registered_delivery(RegisteredDelivery::default())
                .short_message(OctetString::from_str(text)?)
                .build()
                .into_submit_sm(),
        );

        let mut r = self.reader.lock().await;
        let mut w = self.writer.lock().await;

        w.send(&command).await?;

        'outer: while let Some(Ok(command)) = r.next().await {
            match command.pdu() {
                Some(Pdu::SubmitSmResp(_)) => {
                    if let CommandStatus::EsmeRok = command.command_status {
                        break 'outer;
                    } else {
                        return Err(Box::new(SmppClientError::SubmitFailed(
                            command.command_status,
                        )));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn unbind(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let unbind_command = Command::new(CommandStatus::EsmeRok, 3, Pdu::Unbind);
        let mut w = self.writer.lock().await;
        let mut r = self.reader.lock().await;

        w.send(&unbind_command).await?;

        self.connected = false;

        while let Some(Ok(command)) = r.next().await {
            if let CommandId::UnbindResp = command.command_id() {
                if let CommandStatus::EsmeRok = command.command_status {
                    break;
                } else {
                    return Err(Box::new(SmppClientError::UnbindFailed));
                }
            }
        }

        Ok(())
    }
}
