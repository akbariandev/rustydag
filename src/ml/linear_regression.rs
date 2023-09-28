
//const LINEAR_MODEL: &str = "linear";

pub fn create_model() -> String {

    use linfa::traits::{Fit};
    use linfa_linear::LinearRegression;

    let dataset = linfa_datasets::diabetes();
    let lin_reg = LinearRegression::new();
    let model = lin_reg.fit(&dataset).unwrap();
    let precision = 4;

    let label = format!(
        "y = {:.2$}x + {:.2$}",
        model.params()[0],
        model.intercept(),
        precision
    );

    println!("{:?}",label);
    label
}

/*
                    let linear_model = ml::linear_regression::create_model();
                    p2p::add_block(&mut swarm, linear_model);
 */