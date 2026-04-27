# type_uid

This crate defines a `TypUid` macro which can be applied to any Rust structure to generate a unique identifier which depends only on the struct contents and its type. It is useful when creating a serialization/deserialization pipeline, especially using binary formats. During project development, when a type can be modified, it is useful to stamp the serialized data to avoid deserialization errors.

---

## 📊 Usage 

```rust
use type_uid::TypeUid;

#[derive(TypeUid)]
pub struct MyStruct1 {
	field1 : f64,
	field2 : f64,
}

#[derive(TypeUid)]
pub struct MyStruct2 { // identical to MyStruct1
	field1 : f64,
	field2 : f64,
}

#[derive(TypeUid)]
pub struct MyStruct3 {
	field1 : f64,
	field2 : f32, // a modification in the field type has been made
}

pub fn main(){
	let type_signature1 = MyStruct1::TYPE_UID;
	let type_signature2 = MyStruct2::TYPE_UID;
	let type_signature3 = MyStruct3::TYPE_UID;
	
	println!("type_signature1 = {:?}, type_signature2 = {:?}, type_signature3 = {:?}", &type_signature1, &type_signature2, &type_signature3);
}

```

---