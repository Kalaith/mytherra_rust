use super::paginate;

#[test]
fn first_page_starts_at_the_newest() {
    assert_eq!(paginate(30, 14, 0), (0, 0, 14, 3));
}

#[test]
fn a_middle_and_last_page_slice_correctly() {
    assert_eq!(paginate(30, 14, 1), (1, 14, 28, 3));
    assert_eq!(paginate(30, 14, 2), (2, 28, 30, 3));
}

#[test]
fn an_overshot_request_clamps_to_the_last_page() {
    assert_eq!(paginate(30, 14, 99), (2, 28, 30, 3));
}

#[test]
fn an_empty_list_is_one_empty_page() {
    assert_eq!(paginate(0, 14, 3), (0, 0, 0, 1));
}

#[test]
fn a_zero_page_size_never_divides_by_zero() {
    assert_eq!(paginate(5, 0, 0), (0, 0, 1, 5));
}
