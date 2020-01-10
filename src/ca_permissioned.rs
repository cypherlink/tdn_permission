use std::collections::HashMap;
use std::net::SocketAddr;
use tdn::async_std::sync::Sender;
use tdn::prelude::*;
use tdn::traits::group::Group;
use tdn::traits::peer::Peer;

#[derive(Default, Debug)]
pub struct CAPermissionedGroup<P: Peer> {
    id: GroupId,
    my_sk: P::SecretKey,
    my_pk: P::PublicKey,
    my_prove: P::Signature,
    ca: P::PublicKey,
    peers: HashMap<PeerAddr, (P::PublicKey, P::Signature, SocketAddr)>,
}

impl<P: Peer> Group for CAPermissionedGroup<P> {
    type JoinType = (P::PublicKey, P::Signature);
    type JoinResultType = ();
}

impl<P: Peer> CAPermissionedGroup<P> {
    pub fn new(
        id: GroupId,
        my_sk: P::SecretKey,
        my_pk: P::PublicKey,
        my_prove: P::Signature,
        ca: P::PublicKey,
    ) -> Self {
        Self {
            id,
            my_sk,
            my_pk,
            my_prove,
            ca,
            peers: HashMap::new(),
        }
    }

    pub fn id(&self) -> &GroupId {
        &self.id
    }

    /// directly add a peer to group.
    pub fn add(
        &mut self,
        peer_addr: PeerAddr,
        pk: P::PublicKey,
        sign: P::Signature,
        addr: SocketAddr,
    ) {
        self.peers.insert(peer_addr, (pk, sign, addr));
    }

    pub fn join_bytes(&self) -> Vec<u8> {
        bincode::serialize(&(&self.my_pk, &self.my_prove)).unwrap_or(vec![])
    }

    pub fn sign_prove(
        &self,
        pk: &P::PublicKey,
    ) -> Result<P::Signature, Box<dyn std::error::Error>> {
        let pk_bytes = bincode::serialize(pk).unwrap_or(vec![]);
        P::sign(&self.my_sk, &pk_bytes)
    }

    /// join: when peer join will call
    pub async fn join(
        &mut self,
        peer_addr: PeerAddr,
        addr: SocketAddr,
        join_bytes: Vec<u8>,
        return_sender: Sender<Message>,
    ) {
        let is_ok = self.peers.contains_key(&peer_addr);
        if is_ok {
            return_sender
                .send(Message::PeerJoinResult(peer_addr, true, vec![]))
                .await;
        }

        let join_data = bincode::deserialize::<(P::PublicKey, P::Signature)>(&join_bytes);
        if join_data.is_err() {
            return return_sender
                .send(Message::PeerJoinResult(peer_addr, false, vec![]))
                .await;
        }
        let (pk, sign) = join_data.unwrap();
        let pk_bytes = bincode::serialize(&pk).unwrap_or(vec![]);

        if P::verify(&self.ca, &pk_bytes, &sign) {
            let sign_bytes = bincode::serialize(&self.my_prove).unwrap_or(vec![]);
            return_sender
                .send(Message::PeerJoinResult(peer_addr, true, sign_bytes))
                .await;

            self.peers.insert(peer_addr, (pk, sign, addr));
        } else {
            return_sender
                .send(Message::PeerJoinResult(peer_addr, false, vec![]))
                .await;
        }
    }

    pub fn join_result(&mut self, peer_addr: PeerAddr, is_ok: bool, _join_result: Vec<u8>) {
        if !is_ok {
            self.peers.remove(&peer_addr);
        }
    }

    /// leave: when peer leave will call
    pub fn leave(&mut self, peer_addr: &PeerAddr) {
        self.peers.remove(&peer_addr);
    }
}
