use lopdf::content::{Content, Operation};
use lopdf::dictionary;
use lopdf::{Document, Object, Stream};
use rustler::{Atom, Env, Error as RustlerError, NifStruct, NifUnitEnum, Term};
use std::collections::BTreeMap;
use std::io::Error as IoError;
use std::io::ErrorKind as IoErrorKind;

#[derive(Debug, Clone, NifUnitEnum)]
pub enum Field {
    Income,
    Cost,
}

#[derive(Debug, Clone, NifUnitEnum)]
pub enum FieldType {
    Money,
    Text,
    Slotted,
}

#[derive(Debug, NifStruct)]
#[module = "RustlerPdf.PdfWriterOperation"]
pub struct PdfWriterOperation {
    page_number: i32,
    font: (String, i32),
    dimensions: (f64, f64),
    field: Field,
    value: Option<String>,
}

#[derive(Debug, NifStruct)]
#[module = "RustlerPdf.OperationConfig"]
pub struct OperationConfig {
    page_number: i32,
    font: (String, i32),
    field: Field,
    predicate: String,
    field_type: FieldType,
    static_value: Option<String>,
}

#[derive(Debug, NifStruct)]
#[module = "RustlerPdf.PdfWriterConfiguration"]
pub struct PdfWriterConfiguration {
    input_file_path: String,
    output_file_path: String,
    operations: Vec<PdfWriterOperation>,
}

#[rustler::nif]
pub fn r_read_config() -> PdfWriterConfiguration {
    read_config()
}
mod atoms {
    rustler::atoms! {
        ok,
        error,
        eof,

        // Posix
        enoent, // File does not exist
        eacces, // Permission denied
        epipe, // Broken pipe
        eexist, // File exists

        unknown // Other error
    }
}

fn io_error_to_term(err: &IoError) -> Atom {
    match err.kind() {
        IoErrorKind::NotFound => atoms::enoent(),
        IoErrorKind::PermissionDenied => atoms::eacces(),
        IoErrorKind::BrokenPipe => atoms::epipe(),
        IoErrorKind::AlreadyExists => atoms::eexist(),
        _ => atoms::unknown(),
    }
}

#[rustler::nif]
pub fn r_modify_pdf(env: Env, config: PdfWriterConfiguration) -> Result<Term, RustlerError> {
    match modify_pdf(config) {
        Ok(()) => Ok(atoms::ok().to_term(env)),
        Err(ref error) => return Err(RustlerError::Term(Box::new(io_error_to_term(error)))),
    }
}

#[rustler::nif]
pub fn r_create_pdf(env: Env) -> Result<Term, RustlerError> {
    match create_pdf() {
        Ok(()) => Ok(atoms::ok().to_term(env)),
        Err(ref error) => return Err(RustlerError::Term(Box::new(io_error_to_term(error)))),
    }
}

pub fn read_config() -> PdfWriterConfiguration {
    PdfWriterConfiguration {
        input_file_path: "PIT-8C(11)4.pdf".to_string(),
        output_file_path: "PIT-8C-modified.pdf".to_string(),
        operations: vec![
            PdfWriterOperation {
                page_number: 0,
                font: ("F1".to_string(), 10),
                dimensions: (462.82, 55.92),
                value: Some("120.99".to_string()),
                field: Field::Income,
            },
            PdfWriterOperation {
                page_number: 0,
                font: ("F1".to_string(), 10),
                dimensions: (43.32, 347.81),
                value: Some("41.0".to_string()),
                field: Field::Income,
            },
        ],
    }
}

fn generate_pdf_operations(operation: &PdfWriterOperation) -> Vec<Operation> {
    let value = match operation.field {
        Field::Income => operation.value.clone().unwrap(),
        Field::Cost => operation.value.clone().unwrap(),
    };

    let v = value.to_string();
    let split_value: Vec<&str> = v.split(".").collect();

    let whole = split_value.get(0).unwrap().to_string();
    let cents = split_value.get(1);
    let whole_length = whole.len() as f64;

    let whole_x_dimension: f64 = operation.dimensions.0 - ((whole_length - 3.0) * 5.0);

    vec![
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new(
            "Tf",
            vec![
                operation.font.0.clone().into(),
                operation.font.1.clone().into(),
            ],
        ),
        lopdf::content::Operation::new(
            "Td",
            vec![whole_x_dimension.into(), operation.dimensions.1.into()],
        ),
        lopdf::content::Operation::new("Tj", vec![lopdf::Object::string_literal(whole)]),
        lopdf::content::Operation::new("ET", vec![]),
        // Cents
        lopdf::content::Operation::new("BT", vec![]),
        lopdf::content::Operation::new(
            "Tf",
            vec![
                operation.font.0.clone().into(),
                operation.font.1.clone().into(),
            ],
        ),
        lopdf::content::Operation::new(
            "Td",
            vec![
                (operation.dimensions.0 + 30.0).into(),
                operation.dimensions.1.into(),
            ],
        ),
        lopdf::content::Operation::new(
            "Tj",
            vec![lopdf::Object::string_literal(if cents.is_none() {
                "00"
            } else {
                cents.unwrap()
            })],
        ),
        lopdf::content::Operation::new("ET", vec![]),
    ]
}

type FontEncodings = BTreeMap<Vec<u8>, String>;

fn scan_content(
    content: &mut lopdf::content::Content,
    encodings: FontEncodings,
    operation_configs: Vec<OperationConfig>,
) -> Vec<PdfWriterOperation> {
    let mut current_encoding = None;
    let mut search_text = None;
    let mut box_coordinates = None;
    let mut pdf_operations: Vec<PdfWriterOperation> = vec![];

    fn collect_text(text: &mut String, encoding: Option<&str>, operands: &[lopdf::Object]) {
        for operand in operands.iter() {
            match *operand {
                lopdf::Object::String(ref bytes, _) => {
                    let decoded_text = lopdf::Document::decode_text(encoding, bytes);
                    text.push_str(&decoded_text);
                }
                lopdf::Object::Array(ref arr) => {
                    collect_text(text, encoding, arr);
                }
                _ => {}
            }
        }
    }

    for operation in &content.operations {
        match operation.operator.as_ref() {
            "BDC" => {}
            "re" => {
                box_coordinates = Some(
                    operation
                        .operands
                        .clone()
                        .iter()
                        .filter_map(|obj| {
                            let maybe_f32 = lopdf::Object::as_f32(obj);

                            if maybe_f32.is_ok() {
                                Some(maybe_f32.unwrap())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<f32>>(),
                );
            }
            "Tm" => {}
            "Tf" => {
                let a = operation.operands.get(0);
                let current_font = a
                    .ok_or_else(|| lopdf::Error::Syntax("missing font operand".to_string()))
                    .unwrap()
                    .as_name()
                    .unwrap();

                current_encoding = encodings.get(current_font).map(std::string::String::as_str);
            }
            "TJ" => {
                let mut text = String::new();
                collect_text(&mut text, current_encoding, &operation.operands);
                if text.len() > 0 {
                    if text.clone().contains("Wojew") {}
                    search_text = Some(text);
                }
            }
            "Tc" => {}
            "EMC" => {}
            _ => {}
        }

        match (&box_coordinates, &search_text) {
            (Some(cords), Some(text)) => {
                for config in &operation_configs {
                    if text.as_str() == config.predicate {
                        pdf_operations.push(PdfWriterOperation {
                            page_number: config.page_number,
                            font: (config.font.0.to_string(), config.font.1),
                            dimensions: (
                                cords.get(0).unwrap().clone().into(),
                                cords.get(1).unwrap().clone().into(),
                            ),
                            value: if config.static_value.is_some() {
                                config.static_value.clone()
                            } else {
                                Some("Placeholder".to_string())
                            },
                            field: config.field.clone(),
                        })
                    }
                }
                search_text = None;
            }
            _ => {}
        }
    }

    pdf_operations
}

pub fn modify_pdf(config: PdfWriterConfiguration) -> Result<(), std::io::Error> {
    let mut doc = lopdf::Document::load(config.input_file_path).unwrap();
    doc.version = "1.5".to_string();

    // @TODO
    // maybe to solve the issue with polish character, we can try inserting an image instead
    let operation_configs = vec![
        OperationConfig {
            page_number: 0,
            field: Field::Income,
            predicate: "11".to_string(),
            font: ("F1".to_string(), 10),
            field_type: FieldType::Text,
            static_value: Some("127.00".to_string()),
        },
        OperationConfig {
            page_number: 0,
            field: Field::Income,
            predicate: "12".to_string(),
            font: ("F1".to_string(), 10),
            field_type: FieldType::Text,
            static_value: Some("128.00".to_string()),
        },
        OperationConfig {
            page_number: 0,
            field: Field::Income,
            predicate: "23".to_string(),
            font: ("F1".to_string(), 10),
            field_type: FieldType::Money,
            static_value: None,
        },
    ];

    let page_number = 0u32;

    let page_id = doc
        .page_iter()
        .nth(page_number as usize)
        .ok_or(lopdf::Error::PageNumberNotFound(page_number))
        .unwrap();
    let mut page_content = doc.get_and_decode_page_content(page_id).unwrap();
    let encodings = doc
        .get_page_fonts(page_id)
        .into_iter()
        .map(|(name, font)| (name, font.get_font_encoding().to_owned()))
        .collect::<FontEncodings>();

    let operations = scan_content(&mut page_content, encodings, operation_configs);
    for operation in operations {
        let operations = generate_pdf_operations(&operation);
        for op in operations {
            page_content.operations.push(op);
        }

        doc.add_page_contents(page_id, page_content.encode().unwrap())
            .unwrap();
    }

    doc.save(config.output_file_path);
    Ok(())
}

pub fn create_pdf() -> Result<(), std::io::Error> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => "Helvetica",
        "Encoding" => "PDFDocEncoding",
        // "Encoding" => "WinAnsiEncoding"
    });
    let font_id2 = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => "Helvetica",
        // "Encoding" => "WinAnsiEncoding"
        "Encoding" => "MacRomanEncoding"
    });
    let font_id3 = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "TrueType",
        "BaseFont" => "Courier",
        "Encoding" => "MacRomanEncoding"
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
            "F2" => font_id2,
            "F3" => font_id3,
        },
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 10i32.into()]),
            Operation::new("Td", vec![200i32.into(), 200i32.into()]),
            Operation::new("Tj", vec![Object::string_literal("Welcome to Pdfź")]),
            Operation::new("ET", vec![]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F2".into(), 10i32.into()]),
            Operation::new("Td", vec![300i32.into(), 300i32.into()]),
            Operation::new("Tj", vec![Object::string_literal("2 Welcome to Pdf(ź)")]),
            Operation::new("ET", vec![]),
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F3".into(), 10i32.into()]),
            Operation::new("Td", vec![300i32.into(), 400i32.into()]),
            Operation::new("Tj", vec![Object::string_literal("3 Welcome to Pdfź")]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
    });
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
        "Resources" => resources_id,
        "MediaBox" => vec![0i32.into(), 0i32.into(), 595i32.into(), 842i32.into()],
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();

    // Store file in current working directory.
    // Note: Line is exclude for when running tests
    doc.save("example.pdf").unwrap();

    Ok(())
}
rustler::init!(
    "Elixir.RustlerPdf",
    [r_read_config, r_modify_pdf, r_create_pdf]
);
