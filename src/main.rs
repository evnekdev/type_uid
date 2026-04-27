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
