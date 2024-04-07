macro_rules! map {
    ($result:expr) => {
        $result.map_err(pyo3::prelude::PyErr::from)
    };
}

macro_rules! iter_dict_items {
    (for ( $key:ident : $key_ty:ty, $val:ident : $val_ty:ty ) in $dict:expr => $runner:expr) => {
        for ($key, $val) in $dict.iter() {
            let $key = map!($key.downcast::<$key_ty>())?;
            let $val = map!($val.downcast::<$val_ty>())?;
            $runner
        }
    };
}

macro_rules! get_dict_item {
    ($dict:expr, $item:ident as $item_ty:ty) => {
        Some($dict.get_item($item)?.expect("should get item"))
            .as_ref()
            .map(|item| item.downcast::<$item_ty>())
            .transpose()
            .map_err(PyErr::from)?
            .cloned()
    };
}

macro_rules! iter_list_items {
    (for ( $item:ident : $item_ty:ty ) in $list:expr => $runner:expr) => {
        for $item in $list.iter() {
            let $item = map!($item.downcast::<$item_ty>())?;
            $runner
        }
    };
}

macro_rules! iter_mapping_items {
    (for ( $key:ident : $key_ty:ty, $val:ident : $val_ty:ty) in $mapping:expr => $runner:expr) => {
        let items = $mapping.items()?.to_list()?;
        for item in items.iter() {
            let item = map!(item.downcast::<pyo3::types::PyTuple>())?;
            let $key = map!(item.get_borrowed_item(0))?;
            let $key = map!($key.as_any().downcast::<$key_ty>())?;
            let $val = map!(item.get_borrowed_item(1))?;
            let $val = map!($val.as_any().downcast::<$val_ty>())?;
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
