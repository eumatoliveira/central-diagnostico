use bitcoin::{Address, Network, address::NetworkUnchecked};
use std::str::FromStr;

pub fn compare_recipients(reference: &str, destination: &str) -> Result<bool, &'static str> {
    let parse = |input: &str| {
        let address = input.trim();
        if address.is_empty()
            || address.len() > 90
            || !address.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return Err("Informe apenas um endereço público Bitcoin em cada campo.");
        }
        Address::<NetworkUnchecked>::from_str(address)
            .map_err(|_| "Endereço inválido. Confira todos os caracteres e o checksum.")?
            .require_network(Network::Bitcoin)
            .map_err(|_| "O endereço não pertence à rede principal do Bitcoin.")
    };
    Ok(parse(reference)? == parse(destination)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_equal_and_different_mainnet_addresses() {
        let first = "1BoatSLRHtKNngkdXEeobR76b53LETtpyT";
        assert_eq!(compare_recipients(first, first), Ok(true));
        assert_eq!(
            compare_recipients(first, "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy"),
            Ok(false)
        );
        let segwit = "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4";
        assert_eq!(compare_recipients(segwit, &segwit.to_uppercase()), Ok(true));
    }

    #[test]
    fn rejects_checksum_wrong_network_and_private_input_shapes() {
        let valid = "1BoatSLRHtKNngkdXEeobR76b53LETtpyT";
        for invalid in [
            "1BoatSLRHtKNngkdXEeobR76b53LETtpyX",
            "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn",
            "alpha beta gamma",
            "bitcoin:1BoatSLRHtKNngkdXEeobR76b53LETtpyT",
        ] {
            assert!(compare_recipients(valid, invalid).is_err());
        }
    }
}
