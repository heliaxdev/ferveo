#![allow(clippy::many_single_char_names)]
#![allow(non_snake_case)]

use ark_bls12_381::Fr;
use ark_ff::UniformRand;
use ferveo::hybridvss::sh::*;
use ferveo::hybridvss::Params;
use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use rand::Rng;
use rand::SeedableRng;

type Scalar = Fr;

// A HybridVss_sh scheme
pub struct Scheme {
    pub nodes: Vec<Context>,
    pub params: Params,
}

impl Scheme {
    /* Generate a fresh setup with `n` participants,
    failure threshold `f`,
    threshold `t` */
    pub fn new(params: Params) -> Self {
        let nodes = (0..params.n())
            .map(|i| Context::init(params.clone(), i))
            .collect();
        Scheme { nodes, params }
    }

    // get a mutable reference to a node
    fn node_mut(&mut self, i: u32) -> &mut Context {
        &mut self.nodes[i as usize]
    }

    // get a mutable reference to the dealer node
    fn dealer_mut(&mut self) -> &mut Context {
        self.node_mut(self.params.d)
    }

    // dealer responds to a share message
    pub fn dealer_share<R: Rng>(
        &mut self,
        share: Share,
        rng: &mut R,
    ) -> ShareResponse {
        self.dealer_mut().share(rng, share)
    }

    // node i responds to a send message
    fn send(&mut self, i: u32, send: Send) -> SendResponse {
        self.node_mut(i).send(send)
    }

    // respond to a vector of sends, one for each node
    fn send_each(&mut self, sends: Vec<Send>) -> Vec<SendResponse> {
        sends
            .into_iter()
            .enumerate()
            .map(|(i, send)| self.send(i as u32, send))
            .collect()
    }

    // respond to a vector of valid sends, one for each node
    pub fn send_valid_each(&mut self, sends: Vec<Send>) -> Vec<Vec<Echo>> {
        self.send_each(sends)
            .into_iter()
            .map(|echo| echo.unwrap())
            .collect()
    }

    // node i responds to an echo message from m
    fn echo(&mut self, i: u32, m: u32, echo: &Echo) -> EchoResponse {
        self.node_mut(i).echo(m, echo)
    }

    // node i responds to `ceil ((n+t+1)/2)` randomly chosen echos
    fn echo_threshold<R: Rng>(
        &mut self,
        i: u32,
        echos: Vec<&Echo>,
        rng: &mut R,
    ) -> EchoResponse {
        let t = self.params.t;
        let W = self.params.total_weight();
        let threshold = num::integer::div_ceil(W + t + 1, 2) as usize;
        let echos = echos.iter().enumerate().choose_multiple(rng, threshold);
        let mut response = None;
        for (m, echo) in echos {
            response = self.echo(i, m as u32, echo);
        }
        response
    }

    // each node responds to `ceil ((n+t+1)/2)` randomly chosen echos
    pub fn echo_threshold_each<R: Rng>(
        &mut self,
        echos: Vec<Vec<Echo>>,
        rng: &mut R,
    ) -> Vec<EchoResponse> {
        (0..self.params.n())
            .map(|i| {
                let echos: Vec<&Echo> =
                    echos.iter().map(|echos_m| &echos_m[i as usize]).collect();
                self.echo_threshold(i, echos, rng)
            })
            .collect()
    }

    // node i responds to a ready message from m
    fn ready(&mut self, i: u32, m: u32, ready: &Ready) -> ReadyResponse {
        self.node_mut(i).ready(m, ready)
    }

    // node i responds to `n-t-f` randomly chosen ready messages
    fn ready_threshold<R: Rng>(
        &mut self,
        i: u32,
        ready_messages: Vec<&Ready>,
        rng: &mut R,
    ) -> ReadyResponse {
        let Params { t, f, .. } = self.params;
        let W = self.params.total_weight();
        let threshold = (W - t - f) as usize;
        let ready_messages = ready_messages
            .iter()
            .enumerate()
            .choose_multiple(rng, threshold);
        let mut response = None;
        for (m, ready) in ready_messages {
            response = self.ready(i, m as u32, ready);
        }
        response
    }

    // each node responds to `ceil ((n+t+1)/2)` randomly chosen echos
    pub fn ready_threshold_each<R: Rng>(
        &mut self,
        ready_messages: Vec<Vec<Ready>>,
        rng: &mut R,
    ) -> Vec<ReadyResponse> {
        (0..self.params.n())
            .map(|i| {
                let ready_messages: Vec<&Ready> = ready_messages
                    .iter()
                    .map(|ready_messages_m| &ready_messages_m[i as usize])
                    .collect();
                self.ready_threshold(i, ready_messages, rng)
            })
            .collect()
    }
}

#[test]
// test verify_share for valid shares
fn verify_share_valid() {
    let mut rng = StdRng::seed_from_u64(0);
    let w = vec![1; 6];
    let params = Params::random_dealer(0, 4, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };
    let sends = scheme.dealer_share(share, &mut rng);

    sends
        .iter()
        .enumerate()
        .for_each(|(i, send)| assert!(scheme.nodes[i].verify_share(send)));
}

#[test]
// test verify_share for invalid shares
fn verify_share_invalid() {
    let mut rng = StdRng::seed_from_u64(0);
    let w = vec![1; 6];
    let params = Params::random_dealer(0, 4, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };
    let sends = scheme.dealer_share(share, &mut rng);
    /* Reverse the sends to make them all invalid.
    This only works for an even, positive number of sends.
    If there are an odd number of sends,
    the midpoint send will remain valid. */
    sends
        .iter()
        .rev()
        .enumerate()
        .for_each(|(i, send)| assert!(!scheme.nodes[i].verify_share(send)));
}

#[test]
// test that all nodes echo with a valid send
fn send_echo_valid() {
    let mut rng = StdRng::seed_from_u64(0);
    let w = vec![2; 6];
    let params = Params::random_dealer(0, 4, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };
    let sends = scheme.dealer_share(share, &mut rng);
    let responses = scheme.send_each(sends);
    assert!(responses.iter().all(|resp| resp.is_some()))
}

#[test]
// test that all nodes do not echo with an invalid send
fn send_echo_invalid() {
    let mut rng = StdRng::seed_from_u64(0);
    let w = vec![1; 6];
    let params = Params::random_dealer(0, 4, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };

    let sends = scheme.dealer_share(share, &mut rng);
    /* Reverse the sends to make them all invalid.
    This only works for an even, positive number of sends.
    If there are an odd number of sends,
    the midpoint send will remain valid. */
    let sends_rev = sends.into_iter().rev().collect();
    let responses = scheme.send_each(sends_rev);
    assert!(responses.iter().all(|resp| resp.is_none()))
}

#[test]
// test verify_point for valid echo points resulting from sends
fn send_verify_point_valid() {
    let mut rng = StdRng::seed_from_u64(0);
    let n = 8;
    let w = vec![1; n as usize];
    let params = Params::random_dealer(0, 5, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };

    let sends = scheme.dealer_share(share, &mut rng);
    let echos = scheme.send_valid_each(sends);
    for i in 0..n {
        echos.iter().enumerate().for_each(|(m, m_echos)| {
            let Echo { C, alpha } = &m_echos[i];
            assert!(scheme.nodes[i].verify_point(m as u32, C, alpha))
        })
    }
}

#[test]
// test verify_point for invalid echo points resulting from sends
fn send_verify_point_invalid() {
    let mut rng = StdRng::seed_from_u64(0);
    let n = 8;
    let w = vec![1; n as usize];
    let params = Params::random_dealer(0, 5, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };

    let sends = scheme.dealer_share(share, &mut rng);
    let echos = scheme.send_valid_each(sends);
    for i in 0..n {
        echos.iter().enumerate().for_each(|(m, m_echos)| {
            let Echo { C, alpha } = &m_echos[i];
            // should fail with incorrect m
            for not_m in 0..n {
                if not_m != m {
                    assert!(!scheme.nodes[i].verify_point(
                        not_m as u32,
                        C,
                        alpha
                    ))
                }
            }
            // should fail with correct m and mismatched echo
            for not_i in 0..n {
                if not_i != i {
                    let Echo { C, alpha } = &m_echos[not_i];
                    assert!(!scheme.nodes[i].verify_point(m as u32, C, alpha))
                }
            }
        })
    }
}

#[test]
// test that all nodes generate ready messages with enough valid echos
fn echo_ready_threshold() {
    let mut rng = StdRng::seed_from_u64(0);
    let n = 8;
    let w = vec![1; n as usize];
    let params = Params::random_dealer(0, 5, w, &mut rng);
    let mut scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };

    let sends = scheme.dealer_share(share, &mut rng);
    let echos = scheme.send_valid_each(sends);
    for i in 0..n {
        // accept echos from all nodes, in random order
        echos
            .iter()
            .enumerate()
            .choose_multiple(&mut rng, n as usize)
            .into_iter()
            .map(|(m, echos_m)| (m, echos_m[i as usize].clone()))
            .enumerate()
            .for_each(|(count, (m, echo))| {
                let echo_response = scheme.echo(i, m as u32, &echo);
                if count >= 7 - 1 {
                    assert!(echo_response.is_some())
                } else {
                    assert!(echo_response.is_none())
                }
            })
    }
}

#[test]
// test verify_point for valid ready points resulting from echos
fn echo_verify_point_valid() {
    let mut rng = StdRng::seed_from_u64(0);
    let f = 0;
    let n = 8;
    let t = 5;
    let w = vec![1; n as usize];
    let params = Params::random_dealer(f, t, w, &mut rng);
    let scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };

    let mut nodes = scheme.nodes;
    let sends = nodes[scheme.params.d as usize].share(&mut rng, share);
    let echos: Vec<Vec<Echo>> = nodes
        .iter_mut()
        .zip(sends)
        .map(|(node, send)| node.send(send).unwrap())
        .collect();
    // generate ready messages based on a random selection of echos
    let ready_messages: Vec<_> = nodes
        .iter_mut()
        .enumerate()
        .map(|(i, node)| {
            use rand::seq::IteratorRandom;
            let mut res = None;
            echos
                .iter()
                .map(|es| es[i].clone())
                .enumerate()
                .choose_multiple(&mut rng, 7)
                .into_iter()
                .for_each(|(m, echo)| res = node.echo(m as u32, &echo));
            res.expect("Unexpected failure to generate ready message")
        })
        .collect();
    for i in 0..n {
        for (m, m_ready_messages) in ready_messages.iter().enumerate() {
            let ready = &m_ready_messages[i];
            assert!(nodes[i].verify_point(m as u32, &ready.C, &ready.alpha))
        }
    }
}

#[test]
// test that all nodes finish given enough ready messages
fn ready_shared_threshold() {
    let mut rng = StdRng::seed_from_u64(0);
    let f = 0;
    let n = 8;
    let t = 5;
    let w = vec![1; n as usize];
    let params = Params::random_dealer(f, t, w, &mut rng);
    let scheme = Scheme::new(params);
    let share = Share {
        s: Scalar::rand(&mut rng),
    };

    let mut nodes = scheme.nodes;
    let sends = nodes[scheme.params.d as usize].share(&mut rng, share);
    let echos: Vec<Vec<Echo>> = nodes
        .iter_mut()
        .zip(sends)
        .map(|(node, send)| node.send(send).unwrap())
        .collect();
    // generate ready messages based on a random selection of echos
    let ready_messages: Vec<_> = nodes
        .iter_mut()
        .enumerate()
        .map(|(i, node)| {
            use rand::seq::IteratorRandom;
            let mut res = None;
            echos
                .iter()
                .map(|es| es[i].clone())
                .enumerate()
                .choose_multiple(&mut rng, 7)
                .into_iter()
                .for_each(|(m, echo)| res = node.echo(m as u32, &echo));
            res.expect("Unexpected failure to generate ready message")
        })
        .collect();
    nodes.iter_mut().enumerate().for_each(|(i, node)| {
        use rand::seq::IteratorRandom;
        let mut res = None;
        ready_messages
            .iter()
            .map(|rs| rs[i].clone())
            .enumerate()
            .choose_multiple(&mut rng, (n - t - f) as usize)
            .into_iter()
            .for_each(|(m, ready)| {
                assert!(node.verify_point(m as u32, &ready.C, &ready.alpha));
                assert!(res.is_none());
                res = node.ready(m as u32, &ready);
            });
        assert!(res.is_some());
        assert!(res.unwrap().is_right())
    })
}

#[test]
// test share reconstruction
fn reconstruct_share() {
    let mut rng = StdRng::seed_from_u64(0);
    let f = 0;
    let n = 8;
    let t = 5;
    let w = vec![1; n as usize];
    let params = Params::random_dealer(f, t, w, &mut rng);
    let scheme = Scheme::new(params.clone());
    let s = Scalar::rand(&mut rng);
    let mut nodes = scheme.nodes;
    let sends = nodes[scheme.params.d as usize].share(&mut rng, Share { s });
    let echos: Vec<Vec<Echo>> = nodes
        .iter_mut()
        .zip(sends)
        .map(|(node, send)| node.send(send).unwrap())
        .collect();
    // generate ready messages based on a random selection of echos
    let ready_messages: Vec<_> = nodes
        .iter_mut()
        .enumerate()
        .map(|(i, node)| {
            use rand::seq::IteratorRandom;
            let mut res = None;
            echos
                .iter()
                .map(|es| es[i].clone())
                .enumerate()
                .choose_multiple(&mut rng, 7)
                .into_iter()
                .for_each(|(m, echo)| res = node.echo(m as u32, &echo));
            res.expect("Unexpected failure to generate ready message")
        })
        .collect();
    // generate shared messages based on a random selection of ready messages
    let shared_messages: Vec<_> = nodes
        .iter_mut()
        .enumerate()
        .map(|(i, node)| {
            let mut res = None;
            ready_messages
                .iter()
                .enumerate()
                .choose_multiple(&mut rng, (n - t - f) as usize)
                .into_iter()
                .map(|(m, ready_messages_m)| (m, ready_messages_m[i].clone()))
                .into_iter()
                .for_each(|(m, ready_message)| {
                    assert!(res.is_none());
                    res = node.ready(m as u32, &ready_message);
                });
            res.expect("Unexpected failure to generate shared message")
                .expect_right("Unexpected failure to generate shared message")
        })
        .collect();
    // Initialize a rec protocol node
    let i = rng.gen_range(0, 8);
    let mut rec_node = {
        let C = (*shared_messages[i as usize].C).clone();
        let domain = nodes[i].domain;
        let s = shared_messages[i as usize].s;
        ferveo::hybridvss::rec::Context::init(params, C, domain, s)
    };
    // accept T + 1 shares
    let mut z_i = None;
    shared_messages
        .into_iter()
        .enumerate()
        .choose_multiple(&mut rng, (t + 1) as usize)
        .into_iter()
        .for_each(|(j, shared_message)| {
            assert!(z_i.is_none());
            z_i = rec_node.reconstruct_share(j as u32, shared_message.s);
        });
    let z_i = z_i.expect("failed to reconstruct share");
    assert!(z_i == s);
}
