use std::collections::HashMap;

fn add_items(orders: &mut HashMap<u8, Vec<(u8, u8)>>, table: u8, items: &[(u8, u8)]) {
    match orders.get_mut(&table) {
        Some(v) => {
            for item in items { v.push(*item) }
        }
        None => {
            orders.insert(table, items.to_vec()); 
        }
    };
}

fn main() {
    let mut orders: HashMap<u8, Vec<(u8, u8)>> = HashMap::new();
    
    add_items(&mut orders, 1, &[(1,10), (2,15)]);
    add_items(&mut orders, 2, &[(2,15)]);
    add_items(&mut orders, 1, &[(1,5)]);

    println!("{:?}", orders)
}
