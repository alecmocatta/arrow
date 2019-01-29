// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::{
    collections::HashMap,
    fmt::{self, Display},
    marker::PhantomData,
    string::FromUtf8Error,
};

use crate::{
    basic::{LogicalType, Repetition, Type as PhysicalType},
    column::reader::ColumnReader,
    data_type::{ByteArrayType, FixedLenByteArrayType},
    errors::ParquetError,
    record::{
        reader::{ByteArrayReader, FixedLenByteArrayReader, MapReader, Reader},
        schemas::{
            BsonSchema, ByteArraySchema, EnumSchema, FixedByteArraySchema, JsonSchema,
            StringSchema,
        },
        triplet::TypedTripletIter,
        types::{downcast, Value},
        Deserialize,
    },
    schema::types::{ColumnPath, Type},
};

impl Deserialize for Vec<u8> {
    type Reader = ByteArrayReader;
    type Schema = ByteArraySchema;

    fn parse(
        schema: &Type,
        repetition: Option<Repetition>,
    ) -> Result<(String, Self::Schema), ParquetError> {
        Value::parse(schema, repetition).and_then(downcast)
    }

    fn reader(
        _schema: &Self::Schema,
        path: &mut Vec<String>,
        def_level: i16,
        rep_level: i16,
        paths: &mut HashMap<ColumnPath, ColumnReader>,
        batch_size: usize,
    ) -> Self::Reader {
        let col_path = ColumnPath::new(path.to_vec());
        let col_reader = paths.remove(&col_path).unwrap();
        ByteArrayReader {
            column: TypedTripletIter::<ByteArrayType>::new(
                def_level, rep_level, col_reader, batch_size,
            ),
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Bson(Vec<u8>);
impl Deserialize for Bson {
    type Reader = impl Reader<Item = Self>;
    type Schema = BsonSchema;

    fn parse(
        schema: &Type,
        repetition: Option<Repetition>,
    ) -> Result<(String, Self::Schema), ParquetError> {
        Value::parse(schema, repetition).and_then(downcast)
    }

    fn reader(
        schema: &Self::Schema,
        path: &mut Vec<String>,
        def_level: i16,
        rep_level: i16,
        paths: &mut HashMap<ColumnPath, ColumnReader>,
        batch_size: usize,
    ) -> Self::Reader {
        MapReader(
            Vec::<u8>::reader(&schema.0, path, def_level, rep_level, paths, batch_size),
            |x| Ok(Bson(x)),
        )
    }
}
impl From<Bson> for Vec<u8> {
    fn from(json: Bson) -> Self {
        json.0
    }
}
impl From<Vec<u8>> for Bson {
    fn from(string: Vec<u8>) -> Self {
        Bson(string)
    }
}

impl Deserialize for String {
    type Reader = impl Reader<Item = Self>;
    type Schema = StringSchema;

    fn parse(
        schema: &Type,
        repetition: Option<Repetition>,
    ) -> Result<(String, Self::Schema), ParquetError> {
        Value::parse(schema, repetition).and_then(downcast)
    }

    fn reader(
        schema: &Self::Schema,
        path: &mut Vec<String>,
        def_level: i16,
        rep_level: i16,
        paths: &mut HashMap<ColumnPath, ColumnReader>,
        batch_size: usize,
    ) -> Self::Reader {
        MapReader(
            Vec::<u8>::reader(&schema.0, path, def_level, rep_level, paths, batch_size),
            |x| {
                String::from_utf8(x)
                    .map_err(|err: FromUtf8Error| ParquetError::General(err.to_string()))
            },
        )
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Json(String);
impl Deserialize for Json {
    type Reader = impl Reader<Item = Self>;
    type Schema = JsonSchema;

    fn parse(
        schema: &Type,
        repetition: Option<Repetition>,
    ) -> Result<(String, Self::Schema), ParquetError> {
        Value::parse(schema, repetition).and_then(downcast)
    }

    fn reader(
        schema: &Self::Schema,
        path: &mut Vec<String>,
        def_level: i16,
        rep_level: i16,
        paths: &mut HashMap<ColumnPath, ColumnReader>,
        batch_size: usize,
    ) -> Self::Reader {
        MapReader(
            String::reader(&schema.0, path, def_level, rep_level, paths, batch_size),
            |x| Ok(Json(x)),
        )
    }
}
impl Display for Json {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
impl From<Json> for String {
    fn from(json: Json) -> Self {
        json.0
    }
}
impl From<String> for Json {
    fn from(string: String) -> Self {
        Json(string)
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Enum(String);
impl Deserialize for Enum {
    type Reader = impl Reader<Item = Self>;
    type Schema = EnumSchema;

    fn parse(
        schema: &Type,
        repetition: Option<Repetition>,
    ) -> Result<(String, Self::Schema), ParquetError> {
        Value::parse(schema, repetition).and_then(downcast)
    }

    fn reader(
        schema: &Self::Schema,
        path: &mut Vec<String>,
        def_level: i16,
        rep_level: i16,
        paths: &mut HashMap<ColumnPath, ColumnReader>,
        batch_size: usize,
    ) -> Self::Reader {
        MapReader(
            String::reader(&schema.0, path, def_level, rep_level, paths, batch_size),
            |x| Ok(Enum(x)),
        )
    }
}
impl Display for Enum {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}
impl From<Enum> for String {
    fn from(enum_: Enum) -> Self {
        enum_.0
    }
}
impl From<String> for Enum {
    fn from(string: String) -> Self {
        Enum(string)
    }
}

macro_rules! impl_parquet_deserialize_array {
    ($i:tt) => {
        impl Deserialize for [u8; $i] {
            type Reader = impl Reader<Item = Self>;
            type Schema = FixedByteArraySchema<Self>;

            fn parse(
                schema: &Type,
                repetition: Option<Repetition>,
            ) -> Result<(String, Self::Schema), ParquetError> {
                if schema.is_primitive()
                    && repetition == Some(Repetition::REQUIRED)
                    && schema.get_physical_type() == PhysicalType::FIXED_LEN_BYTE_ARRAY
                    && schema.get_basic_info().logical_type() == LogicalType::NONE
                    && schema.get_type_length() == $i
                {
                    return Ok((
                        schema.name().to_owned(),
                        FixedByteArraySchema(PhantomData),
                    ));
                }
                Err(ParquetError::General(format!(
                    "Can't parse array {:?}",
                    schema
                )))
            }

            fn reader(
                _schema: &Self::Schema,
                path: &mut Vec<String>,
                def_level: i16,
                rep_level: i16,
                paths: &mut HashMap<ColumnPath, ColumnReader>,
                batch_size: usize,
            ) -> Self::Reader {
                let col_path = ColumnPath::new(path.to_vec());
                let col_reader = paths.remove(&col_path).unwrap();
                MapReader(
                    FixedLenByteArrayReader {
                        column: TypedTripletIter::<FixedLenByteArrayType>::new(
                            def_level, rep_level, col_reader, batch_size,
                        ),
                    },
                    |bytes: Vec<_>| {
                        let mut ret = std::mem::MaybeUninit::<Self>::uninit();
                        assert_eq!(bytes.len(), unsafe { &*ret.as_ptr() }.len());
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                bytes.as_ptr(),
                                ( &mut *ret.as_mut_ptr() ).as_mut_ptr(),
                                bytes.len(),
                            )
                        };
                        Ok(unsafe { ret.assume_init() })
                    },
                )
            }
        }

        impl Deserialize for Box<[u8; $i]> {
            type Reader = impl Reader<Item = Self>;
            type Schema = FixedByteArraySchema<[u8; $i]>;

            fn parse(
                schema: &Type,
                repetition: Option<Repetition>,
            ) -> Result<(String, Self::Schema), ParquetError> {
                <[u8; $i]>::parse(schema, repetition)
            }

            fn reader(
                _schema: &Self::Schema,
                path: &mut Vec<String>,
                def_level: i16,
                rep_level: i16,
                paths: &mut HashMap<ColumnPath, ColumnReader>,
                batch_size: usize,
            ) -> Self::Reader {
                let col_path = ColumnPath::new(path.to_vec());
                let col_reader = paths.remove(&col_path).unwrap();
                MapReader(
                    FixedLenByteArrayReader {
                        column: TypedTripletIter::<FixedLenByteArrayType>::new(
                            def_level, rep_level, col_reader, batch_size,
                        ),
                    },
                    |bytes: Vec<_>| {
                        let mut ret = box [0u8; $i];
                        assert_eq!(bytes.len(), ret.len());
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                bytes.as_ptr(),
                                ret.as_mut_ptr(),
                                bytes.len(),
                            )
                        };
                        Ok(ret)
                    },
                )
            }
        }
    };
}

// Implemented on common array lengths, copied from arrayvec
impl_parquet_deserialize_array!(0);
impl_parquet_deserialize_array!(1);
impl_parquet_deserialize_array!(2);
impl_parquet_deserialize_array!(3);
impl_parquet_deserialize_array!(4);
impl_parquet_deserialize_array!(5);
impl_parquet_deserialize_array!(6);
impl_parquet_deserialize_array!(7);
impl_parquet_deserialize_array!(8);
impl_parquet_deserialize_array!(9);
impl_parquet_deserialize_array!(10);
impl_parquet_deserialize_array!(11);
impl_parquet_deserialize_array!(12);
impl_parquet_deserialize_array!(13);
impl_parquet_deserialize_array!(14);
impl_parquet_deserialize_array!(15);
impl_parquet_deserialize_array!(16);
impl_parquet_deserialize_array!(17);
impl_parquet_deserialize_array!(18);
impl_parquet_deserialize_array!(19);
impl_parquet_deserialize_array!(20);
impl_parquet_deserialize_array!(21);
impl_parquet_deserialize_array!(22);
impl_parquet_deserialize_array!(23);
impl_parquet_deserialize_array!(24);
impl_parquet_deserialize_array!(25);
impl_parquet_deserialize_array!(26);
impl_parquet_deserialize_array!(27);
impl_parquet_deserialize_array!(28);
impl_parquet_deserialize_array!(29);
impl_parquet_deserialize_array!(30);
impl_parquet_deserialize_array!(31);
impl_parquet_deserialize_array!(32);
impl_parquet_deserialize_array!(40);
impl_parquet_deserialize_array!(48);
impl_parquet_deserialize_array!(50);
impl_parquet_deserialize_array!(56);
impl_parquet_deserialize_array!(64);
impl_parquet_deserialize_array!(72);
impl_parquet_deserialize_array!(96);
impl_parquet_deserialize_array!(100);
impl_parquet_deserialize_array!(128);
impl_parquet_deserialize_array!(160);
impl_parquet_deserialize_array!(192);
impl_parquet_deserialize_array!(200);
impl_parquet_deserialize_array!(224);
impl_parquet_deserialize_array!(256);
impl_parquet_deserialize_array!(384);
impl_parquet_deserialize_array!(512);
impl_parquet_deserialize_array!(768);
impl_parquet_deserialize_array!(1024);
impl_parquet_deserialize_array!(2048);
impl_parquet_deserialize_array!(4096);
impl_parquet_deserialize_array!(8192);
impl_parquet_deserialize_array!(16384);
impl_parquet_deserialize_array!(32768);
impl_parquet_deserialize_array!(65536);