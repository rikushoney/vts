macro_rules! iter_dict_items {
    (for ( $key:ident : $key_ty:ty, $val:ident : $val_ty:ty ) in $dict:expr => $runner:expr) => {
        for ($key, $val) in $dict.iter() {
            let $key = $key.downcast::<$key_ty>()?;
            let $val = $val.downcast::<$val_ty>()?;
            $runner
        }
    };
}

macro_rules! iter_list_items {
    (for ( $item:ident : $item_ty:ty ) in $list:expr => $runner:expr) => {
        for $item in $list.iter() {
            let $item = $item.downcast::<$item_ty>()?;
            $runner
        }
    };
}

macro_rules! iter_mapping_items {
    (for ( $key:ident : $key_ty:ty, $val:ident : $val_ty:ty) in $mapping:expr => $runner:expr) => {
        let items = $mapping.items()?.to_list()?;
        for item in items.iter() {
            let item = item.downcast::<pyo3::types::PyTuple>()?;
            let $key = item.get_borrowed_item(0)?;
            let $key = $key.as_any().downcast::<$key_ty>()?;
            let $val = item.get_borrowed_item(1)?;
            let $val = $val.as_any().downcast::<$val_ty>()?;
            $runner
        }
    };
}

// macro_rules! map_py_de_err {
//     ($expr:expr) => {
//         ($expr).map_err(|err| de::Error::custom(err.to_string()))
//     };
// }

// macro_rules! map_py_ser_err {
//     ($expr:expr) => {
//         ($expr).map_err(|err| ser::Error::custom(err.to_string()))
//     };
// }

macro_rules! map_serde_py_err {
    ($expr:expr) => {
        ($expr).map_err(|err| pyo3::exceptions::PyValueError::new_err(format!("{err}")))
    };
}
