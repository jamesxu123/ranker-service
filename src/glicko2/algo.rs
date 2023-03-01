const TAU: f64 = 0.5;
const FACTOR: f64 = 173.7178;
const EPSILON: f64 = 0.000001;

pub struct Glicko1 {
    pub rating: f64,
    pub sigma: f64,
    pub rd: f64
}

pub struct Glicko2 {
    pub mu: f64,
    pub sigma: f64,
    pub phi: f64
}

impl Glicko2 {
    pub fn from_glicko1(g1: &Glicko1) -> Self{
        Glicko2 { mu: (g1.rating - 1500.0) / FACTOR, sigma: g1.sigma, phi: g1.rd / FACTOR}
    }

    fn update_glicko2_vars(self, phi_star: f64, v: f64, g_opponents: &Vec<&Glicko2>, scores: &Vec<f64>, sigma_prime: f64) -> Glicko2 {
        let phi = 1f64 / (1f64 / phi_star.powi(2) + 1f64 / v).sqrt();
        let mu = self.mu + phi.powi(2) * compute_delta(&self, g_opponents, scores);
        Glicko2 { mu, sigma: sigma_prime, phi}
    }

    pub fn process_matches(self, g_opponents: &Vec<&Glicko2>, scores: &Vec<f64>) -> Glicko2 {
        assert_eq!(g_opponents.len(), scores.len());

        let delta = compute_delta(&self, &g_opponents, scores);
        let v = compute_v(&self, g_opponents);
        let sigma_prime = sigma_by_illinois(&self, delta, v);
        let phi_star = get_new_rating_dev(&self, sigma_prime);
        self.update_glicko2_vars(phi_star, v, g_opponents, scores, sigma_prime)
    }
}

impl Glicko1 {
    pub fn from_glicko2(g2: &Glicko2) -> Self{
        Glicko1 { rating: g2.mu * FACTOR + 1500.0, sigma: g2.sigma, rd: g2.phi * FACTOR }
    }
}

fn e(mu: f64, mu_j: f64, phi_j: f64) -> f64 {
	1.0 / (1.0 + (-g(phi_j)*(mu-mu_j)).exp())
}

fn g(phi: f64) -> f64 {
	1.0 / (1.0 + 3.0 * phi.powi(2) / std::f64::consts::PI.powi(2)).sqrt()
}

fn compute_v(g_cur: &Glicko2, g_opponents: &Vec<&Glicko2>) -> f64 {
    let mut sum: f64 = 0f64;
    for g_op in g_opponents {
        sum += g(g_op.phi).powi(2) * e(g_cur.mu, g_op.mu, g_op.phi) * (1f64 - e(g_cur.mu, g_op.mu, g_op.phi))
    }
    1f64 / sum
}

fn compute_delta(g_cur: &Glicko2, g_opponents: &Vec<&Glicko2>, scores: &Vec<f64>) -> f64 {
    let mut sum = 0f64;
    assert_eq!(g_opponents.len(), scores.len());

    for j in 0..scores.len() {
        let g_op = g_opponents[j];
        sum += g(g_op.phi) * (scores[j] - e(g_cur.mu, g_op.mu, g_op.phi));
    }

    compute_v(g_cur, g_opponents) * sum
}

fn sigma_by_illinois(g_cur: &Glicko2, delta: f64, v: f64) -> f64 {
    let a = g_cur.sigma.powi(2).ln();
    let f = |x: f64| -> f64 {
        let t1: f64 = x.exp() * (delta.powi(2) - g_cur.phi.powi(2) - v - x.exp()) / 2f64 * (g_cur.phi.powi(2) + v + x.exp()).powi(2);
        let t2 = (x - a) / TAU.powi(2);
        t1 - t2
    };

    let mut a_prime = a;
    let phi_sq_plus_v = g_cur.phi.powi(2) + v;

    let mut b_prime = if delta.powi(2) > phi_sq_plus_v {
        (delta.powi(2) - phi_sq_plus_v).ln()
    } else {
        let mut k = 1f64;
        while f(a - TAU * k) < 0.0 {
            k += 1f64
        }
        a - k*TAU
    };

    let mut f_a = f(a_prime);
    let mut f_b = f(b_prime);

    while (b_prime - a_prime).abs() > EPSILON {
        let c = a_prime + (a_prime-b_prime)*f_a/(f_b-f_a);
        let f_c = f(c);
        let fc_fb = f_c * f_b;
        if fc_fb <= 0f64 {
            a_prime = b_prime;
            f_a = f_b;
        } else {
            f_a /= 2f64;
        }
        b_prime = c;
        f_b = f_c;
    }

    (a_prime / 2f64).exp()
}

fn get_new_rating_dev(g_cur: &Glicko2, sigma_prime: f64) -> f64 {
    (g_cur.phi.powi(2) + sigma_prime.powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_all() {
        let p1 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1500f64,
            sigma: 0.06f64,
            rd: 200f64
        });
    
        let o1 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1400f64,
            sigma: 0.06f64,
            rd: 30f64
        });
    
        let o2 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1550f64,
            sigma: 0.06f64,
            rd: 100f64
        });
    
        let o3 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1700f64,
            sigma: 0.06f64,
            rd: 300f64
        });
    
        let scores: Vec<f64> = vec![1f64, 0f64, 0f64];
        let opps = vec![&o1, &o2, &o3];
    
        let pf = p1.process_matches(&opps, &scores);
        let as_g1 = Glicko1::from_glicko2(&pf);
    
        // println!("{:.2}, {:.2},{:.2}", as_g1.rating, as_g1.sigma, as_g1.rd);
        assert!((as_g1.rating - 1436.05).abs() < 0.1);
        assert!((as_g1.sigma - 0.06).abs() < 0.1);
        assert!((as_g1.rd - 151.52).abs() < 0.1);
    }

    #[test]
    fn test_g() {
        let g_val = g(0.5);
        assert!((g_val - 0.96404).abs() < 0.001)
    }

    #[test]
    fn test_e() {
        let e_val = e(0.6, 0.5, 0.5);
        assert!((e_val - 0.52408).abs() < 0.001)
    }

    #[test]
    fn test_compute_delta() {
        let p1: Glicko2 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1500f64,
            sigma: 0.06f64,
            rd: 200f64
        });
    
        let o1: Glicko2 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1400f64,
            sigma: 0.06f64,
            rd: 30f64
        });
    
        let o2: Glicko2 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1550f64,
            sigma: 0.06f64,
            rd: 100f64
        });
    
        let o3: Glicko2 = Glicko2::from_glicko1(&Glicko1 {
            rating: 1700f64,
            sigma: 0.06f64,
            rd: 300f64
        });
    
        let scores: Vec<f64> = vec![1f64, 0f64, 0f64];
        let opps = vec![&o1, &o2, &o3];
    
        let delta = compute_delta(&p1, &opps, &scores);
        assert!((delta + 0.483933260).abs() < 0.001)
    }
}