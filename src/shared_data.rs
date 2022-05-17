use std::collections::HashMap;

use rand::Rng;

#[derive(Serialize, Deserialize, Debug)]
pub struct Order {
    order_id: u64,
    item: String,
    mins_to_cook: u8,
}

impl Order {
    pub fn get_order_id(&self) -> u64 {
        self.order_id
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SharedData {
    order_counter: u64,
    tables: HashMap<u64, Vec<Order>>,
}

impl SharedData {
    pub fn new() -> SharedData {
        SharedData { order_counter: 0, tables: HashMap::new() }
    }

    pub fn get_tables(&self) -> &HashMap<u64, Vec<Order>> {
        &self.tables
    }

    pub fn add_items(&mut self, table: u64, items: Vec<String>) {
        let mut rng = rand::thread_rng();
        let mut orders = items.iter().map(|item| {
            let order = Order {
                order_id: self.order_counter,
                item: item.to_string(),
                mins_to_cook:  rng.gen_range(5..16)
            };
            self.order_counter += 1;
            return order
        }).collect::<Vec<Order>>();

        match self.tables.get_mut(&table) {
            Some(v) => {
                v.append(&mut orders)
            }
            None => {     
                self.tables.insert(table, orders);  
            }
        };
    }

    pub fn remove_item(&mut self, table: u64, order_id: u64) -> Option<String> {
        let orders = self.tables.get_mut(&table);

        let orders = match orders {
            Some(orders) => orders,
            None => return Some(format!("order {} doesn't exist at table {}", order_id, table).to_string())
        };

        let index = orders.iter().position(|o| o.order_id == order_id);

        match index {
            Some(index) => {
                orders.remove(index);
                if orders.len() == 0 { self.tables.remove(&table); }
                None
            },
            None => Some(format!("order {} doesn't exist at table {}", order_id, table).to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{SharedData, Order};

    #[test]
    fn test_add_items() {
        let mut shared_data1 = SharedData::new();
       
        shared_data1.add_items(1, vec!["pizza".to_string(), "tea".to_string()]);

        let shared_data2 = SharedData {
            order_counter: 2, 
            tables: HashMap::from([
                (1, vec![Order {
                    order_id: 0,
                    item: String::from("pizza"),
                    mins_to_cook: 0
                }, Order {
                    order_id: 1,
                    item: String::from("tea"),
                    mins_to_cook: 0
                }]),
            ]) 
        };

        check_orders_for_table(&shared_data1, &shared_data2, 1);
    }

    #[test]
    fn test_remove_items() {
        let mut shared_data1 = SharedData {
            order_counter: 2, 
            tables: HashMap::from([
                (1, vec![Order {
                    order_id: 0,
                    item: String::from("pizza"),
                    mins_to_cook: 0
                }, Order {
                    order_id: 1,
                    item: String::from("tea"),
                    mins_to_cook: 0
                }]),
            ]) 
        };

        shared_data1.remove_item(1, 0);

        let shared_data2 = SharedData {
            order_counter: 2,
            tables: HashMap::from([
                (1, vec![Order {
                    order_id: 1,
                    item: String::from("tea"),
                    mins_to_cook: 0
                }]),
            ]) 
        };

        check_orders_for_table(&shared_data1, &shared_data2, 1);

        shared_data1.remove_item(1, 1);
        assert!(shared_data1.tables.is_empty(), "tables should be empty! Current length: {}", shared_data1.tables.len());
    }

    fn check_orders_for_table(shared_data1: &SharedData, shared_data2: &SharedData, table: u64) {
        assert_eq!(
            shared_data1.order_counter, 
            shared_data2.order_counter, 
            "unequal order counts: {}, {}", 
            shared_data1.order_counter,
            shared_data2.order_counter
        );

        let orders1 = shared_data1.tables.get(&table).unwrap();
        let orders2 = shared_data2.tables.get(&table).unwrap();

        assert_eq!(orders1.len(), orders2.len(), "unequal lengths: {}, {}", orders1.len(), orders2.len());

        for (order1, order2) in orders1.iter().zip(orders2.iter()) {
            assert_eq!(order1.order_id, order2.order_id, "different order_ids: {}, {}", order1.order_id, order2.order_id);
            assert_eq!(order1.item, order2.item, "different items: {}, {}", order1.item, order2.item);
        }
    }
}