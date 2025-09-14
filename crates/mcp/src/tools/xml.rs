use quick_xml::events::{BytesCData, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::writer::Writer;
use std::io::Cursor;

/// Trait for converting tool output structures to XML
pub trait ToXml {
    /// Convert the structure to XML string with proper formatting
    fn to_xml(&self) -> Result<String, Box<dyn std::error::Error>>;
}

pub struct XmlBuilder {
    writer: Writer<Cursor<Vec<u8>>>,
}

impl XmlBuilder {
    pub fn new() -> Self {
        let writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        Self { writer }
    }

    /// Start a new XML element
    pub fn start_element(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let elem = BytesStart::new(name);
        self.writer.write_event(Event::Start(elem))?;
        Ok(())
    }

    /// End the current XML element
    pub fn end_element(&mut self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let elem = BytesEnd::new(name);
        self.writer.write_event(Event::End(elem))?;
        Ok(())
    }

    pub fn write_element(
        &mut self,
        name: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_element(name)?;
        let text = BytesText::new(content);
        self.writer.write_event(Event::Text(text))?;
        self.end_element(name)?;
        Ok(())
    }

    pub fn write_cdata_element(
        &mut self,
        name: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_element(name)?;
        let cdata = Event::CData(BytesCData::new(format!("\n{}\n", content)));
        self.writer.write_event(cdata)?;
        self.end_element(name)?;
        Ok(())
    }

    pub fn write_optional_element(
        &mut self,
        name: &str,
        content: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(content) = content {
            self.write_element(name, content)?;
        }
        Ok(())
    }

    pub fn write_optional_cdata_element(
        &mut self,
        name: &str,
        content: &Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(content) = content {
            self.write_cdata_element(name, content)?;
        }
        Ok(())
    }

    pub fn write_numeric_element<T: std::fmt::Display>(
        &mut self,
        name: &str,
        value: T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write_element(name, &value.to_string())
    }

    pub fn write_optional_numeric_element<T: std::fmt::Display>(
        &mut self,
        name: &str,
        value: &Option<T>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(value) = value {
            self.write_numeric_element(name, value)?;
        }
        Ok(())
    }

    pub fn write_boolean_element(
        &mut self,
        name: &str,
        value: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.write_element(name, if value { "true" } else { "false" })
    }

    pub fn finish(self) -> Result<String, Box<dyn std::error::Error>> {
        let inner = self.writer.into_inner();
        let bytes = inner.into_inner();
        Ok(String::from_utf8(bytes)?)
    }
}

impl Default for XmlBuilder {
    fn default() -> Self {
        Self::new()
    }
}
