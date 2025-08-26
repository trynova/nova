/// Creates a [`SoAVec`] containing the arguments.
///
/// `soavec!` allows `SoAVec`s to be defined with the same syntax as array
/// expressions. There are two forms of this macro:
///
/// - Create a [`SoAVec`] containing a given list of elements:
///
/// ```
/// use nova_vec::soavec;
///
/// let v = soavec![(1, 1), (2, 2), (3, 3)].unwrap();
/// assert_eq!(v.get(0), Some((&1, &1)));
/// assert_eq!(v.get(1), Some((&2, &2)));
/// assert_eq!(v.get(2), Some((&3, &3)));
/// ```
///
/// - Create a [`SoAVec`] from a given element and size:
///
/// ```
/// use nova_vec::soavec;
///
/// let v = soavec![(1, 1); 3].unwrap();
/// assert_eq!(v.get(0), Some((&1, &1)));
/// assert_eq!(v.get(1), Some((&1, &1)));
/// assert_eq!(v.get(2), Some((&1, &1)));
/// ```
///
/// Note that unlike array expressions this syntax supports all elements
/// which implement [`Clone`] and the number of elements doesn't have to be
/// a constant.
///
/// This will use `clone` to duplicate an expression, so one should be careful
/// using this with types having a nonstandard `Clone` implementation. For
/// example, `soavec![Rc::new(1); 5]` will create a vector of five references
/// to the same boxed integer value, not five references pointing to independently
/// boxed integers.
///
/// Also, note that `soavec![expr; 0]` is allowed, and produces an empty vector.
/// This will still evaluate `expr`, however, and immediately drop the resulting value, so
/// be mindful of side effects.
///
/// [`SoAVec`]: crate::SoAVec
#[macro_export]
macro_rules! soavec {
    () => (
        $crate::SoAVec::new()
    );
    ($elem:expr; $n:expr) => (
        // $crate::vec::from_elem($elem, $n)
        $crate::SoAVec::with_capacity(
            u32::try_from($n).expect("SoAVec cannot handle more than u32::MAX items")
        ).map(|mut vec| {
            let mut i = 0;
            while i < $n {
                // SAFETY: pre-reserved with capacity to hold temp.
                unsafe { vec.push($elem).unwrap_unchecked() };
                i += 1;
            }
            vec
        })
    );
    ($($x:expr),+ $(,)?) => (
        $crate::SoAVec::with_capacity(
            u32::try_from([$($x),+].len()).expect("SoAVec cannot handle more than u32::MAX items")
        ).map(|mut vec| {
            let data = [$($x),+];
            let len = data.len() as u32;
            let mut i = 0;
            while i < len {
                // SAFETY: pre-reserved with capacity to hold temp.
                unsafe { vec.push(data[i as usize]).unwrap_unchecked() };
                i += 1;
            }
            vec
        })
    );
}
