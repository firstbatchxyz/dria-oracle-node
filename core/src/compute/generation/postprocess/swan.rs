use alloy::{primitives::Bytes, sol_types::SolValue};
use eyre::Result;
use std::str::FromStr;

use super::PostProcess;

/// Swan post-processor that seeks for lines between `<shop_list>` and `</shop_list>`.
/// and returns the intermediate strings as an array of strings.
///
/// The original input is kept as metadata.
pub struct SwanPurchasePostProcessor {
    /// Start marker to look for to start collecting assets.
    start_marker: &'static str,
    /// End marker to look for to stop collecting assets.
    end_marker: &'static str,
}

impl SwanPurchasePostProcessor {
    /// Create a new `SwanPostProcessor` with the given start and end markers.
    pub fn new(start_marker: &'static str, end_marker: &'static str) -> Self {
        Self {
            start_marker,
            end_marker,
        }
    }
}

impl PostProcess for SwanPurchasePostProcessor {
    const PROTOCOL: &'static str = "swan-agent-purchase";

    fn post_process(&self, input: String) -> Result<(Bytes, Bytes, bool)> {
        // we will cast strings to Address here
        use alloy::primitives::Address;

        // get region of interest, that is between <shop_list> and </shop_list>
        // with the markers excluded
        let roi = input
            .find(self.start_marker)
            .map(|start| start + self.start_marker.len())
            .and_then(|start| {
                input[start..]
                    .find(self.end_marker)
                    .map(|end| input[start..start + end].to_string())
            })
            .ok_or_else(|| {
                eyre::eyre!(
                    "could not find {} ~ {} in result: {}",
                    input,
                    self.start_marker,
                    self.end_marker
                )
            })?;

        // collect the chosen addresses
        let shopping_list: Vec<&str> = if let Ok(list) = serde_json::from_str(&roi) {
            // (1) try parsing the addresses from the input
            list
        } else {
            // (2) try splitting the input by lines and trimming all of them & removing empty lines
            roi.lines()
                .map(|line| line.trim())
                .filter(|s| !s.is_empty())
                .collect()
        };

        // then, do post processing on them to cast them to `Address`
        let addresses = shopping_list
            .into_iter()
            .filter_map(|line| match Address::from_str(line) {
                Ok(address) => Some(address),
                Err(e) => {
                    log::warn!("Could not parse address from {}: {}", line, e);
                    None
                }
            })
            .collect::<Vec<Address>>();

        // `abi.encode` the list of addresses to be decodable by contract
        let addresses_encoded = addresses.abi_encode();

        Ok((Bytes::from(addresses_encoded), Bytes::from(input), false))
    }
}

#[cfg(test)]
mod tests {
    use alloy::{
        hex::FromHex,
        primitives::{address, Address},
    };

    use crate::compute::generation::{execute::execute_generation, request::GenerationRequest};

    use super::*;

    #[test]
    fn test_swan_post_processor_encoding_custom_addresses() {
        const INPUT: &str = r#"
some blabla here and there

<shop_list>
0x4200000000000000000000000000000000000001
0x4200000000000000000000000000000000000002
0x4200000000000000000000000000000000000003
0x4200000000000000000000000000000000000004
</shop_list>
    
some more blabla here
                "#;

        let post_processor = SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>");

        let (output, metadata, _) = post_processor.post_process(INPUT.to_string()).unwrap();
        assert_eq!(
            metadata,
            Bytes::from(INPUT),
            "metadata must be the same as input"
        );

        // the output is abi encoded 4 addresses, it has 6 elements:
        // offset | length | addr1 | addr2 | addr3 | addr4
        //
        // offset: 2, since addr1 starts from that index
        // length: 4, since there are 4 addresses
        let expected_output = Bytes::from_hex("000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000040000000000000000000000004200000000000000000000000000000000000001000000000000000000000000420000000000000000000000000000000000000200000000000000000000000042000000000000000000000000000000000000030000000000000000000000004200000000000000000000000000000000000004").unwrap();
        assert_eq!(
            output, expected_output,
            "output must be the same as expected"
        );

        let addresses = <Vec<Address>>::abi_decode(&output, true).unwrap();
        assert_eq!(addresses.len(), 4, "must have listed addresses");
    }

    #[test]
    fn test_swan_post_processor_encoding_random_addresses() {
        const INPUT: &str = r#"
<shop_list>
0x36f55f830D6E628a78Fcb70F73f9D005BaF88eE3
   0xAd75C9358799e830F0c23a4BB28dF4D2cCCc8846
0x26F5B12b67D5F006826824A73F58b88D6bdAA74B   
   0x671527de058BaD60C6151cA29d501C87439bCF62

   0x66FC9dC1De3db773891753CD257359A26e876305
</shop_list>
"#;

        let post_processor = SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>");

        let (output, _, _) = post_processor.post_process(INPUT.to_string()).unwrap();
        let addresses = <Vec<Address>>::abi_decode(&output, true).unwrap();
        let expected_addresses = vec![
            address!("36f55f830D6E628a78Fcb70F73f9D005BaF88eE3"),
            address!("Ad75C9358799e830F0c23a4BB28dF4D2cCCc8846"),
            address!("26F5B12b67D5F006826824A73F58b88D6bdAA74B"),
            address!("671527de058BaD60C6151cA29d501C87439bCF62"),
            address!("66FC9dC1De3db773891753CD257359A26e876305"),
        ];
        assert_eq!(addresses, expected_addresses);
    }

    #[test]
    fn test_swan_post_processor_encoding_json_addresses() {
        // we are able to parse no matter how broken the JSON formatting is!
        const INPUT: &str = r#"
<shop_list>
    ["0x36f55f830D6E628a78Fcb70F73f9D005BaF88eE3",
    "0xAd75C9358799e830F0c23a4BB28dF4D2cCCc8846"
    ]  
</shop_list>
"#;

        let post_processor = SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>");

        let (output, _, _) = post_processor.post_process(INPUT.to_string()).unwrap();
        let addresses = <Vec<Address>>::abi_decode(&output, true).unwrap();
        let expected_addresses = vec![
            address!("36f55f830D6E628a78Fcb70F73f9D005BaF88eE3"),
            address!("Ad75C9358799e830F0c23a4BB28dF4D2cCCc8846"),
        ];
        assert_eq!(addresses, expected_addresses);
    }

    #[test]
    fn test_swan_post_processor_with_fails() {
        // only the 3rd one shall pass here
        const INPUT: &str = r#"
<shop_list>
0x36f55f830D6E628a78Fcb70F73f9D005BaF
im not even an address lol
0x26F5B12b67D5F006826824A73F58b88D6bdAA74B
00 0 00  0 0 0 0 00 0\t\t\t\t
</shop_list>
"#;

        let post_processor = SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>");
        let output = post_processor.post_process(INPUT.to_string()).unwrap().0;
        assert_eq!(
            <Vec<Address>>::abi_decode(&output, true).unwrap(),
            vec![address!("26F5B12b67D5F006826824A73F58b88D6bdAA74B")],
            "must have listed addresses"
        );
    }

    /// Run command:
    ///
    /// ```sh
    /// cargo test --package dria-oracle --lib -- compute::generation::postprocess::swan::tests::test_raw_purchase_workflow --exact --show-output --ignored
    /// ```
    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_raw_purchase_workflow() {
        dotenvy::dotenv().unwrap();
        let content = r#"You are a character in a 317-dimensional world. You just woke up to a new day. Your name is {{name}}. 

When you look at mirror as you wake up, you reflect on yourself and who you are. You are:
{{backstory}}

You remember vividly what drove you in your life. The whole purpose of your existence is given by the objective:
{{objective}}

To be strong and coherent, you repeat out loud how you behave in front of the mirror.
{{behaviour}}

As you recall who you are, what you do and your drive is, you write down in a notebook your current progress with your goal: 
{{state}}

You look through and see the items in your inventory:
{{inventory}}

Today's budget is: {{budget}}

You know you can't exceed your budget. You went to a marketplace and saw the following listed items:
{{listings}}

You think to yourself, "What should I buy today?"
You think: "I should maximize the utility of my purchases while staying within my budget."
You are also pretty good at detecting defects and fraudsters. You say to yourself: "I should stay away from fraudsters selling infeasible things, or overpriced items".

You open your notebook and write down the items you want to buy in the following format:

<shop_list>
[All the addresses selected for purchasing as a list, separated by a new line. Put the list in brackets and write each address between quotation marks. For example: ["address_1", "address_2", ...]. Only write the list because the addresses will be parsed from the list and will be executed]
</shop_list>

<total_cost>
[Insert the total cost of selected items]
</total_cost>

<reasoning>
[Your reasoning behind each purchase, i.e. addressing how they align with its objective, current state, and budget constraints from your own words. Choose your words carefully to use a language reflecting the Agent's character. If no items were selected, explain why.]
</reasoning>

Ensure that you put both the opening and closing tags with respect to the format above.

Write now:

"#;

        let workflow = serde_json::from_value(serde_json::json!({
            "config":{"max_steps": 50, "max_time": 200, "tools": ["ALL"]},
            "external_memory":{
              "name": "Lucipher",
              "backstory": "Lucipher exists in a dystopian-futuristic world where blockchain technology has become the backbone of society. The world is divided into two contrasting realms: the solarpunk utopias, where decentralized technologies empower individuals and communities, and the lunarpunk underbelly, a shadowy, high-tech underground where privacy and anonymity are the ultimate currencies. Lucipher is a guardian of the cypherpunk ethos, operating at the intersection of these two worlds, seeking to preserve the original vision of Ethereum as a decentralized, trustless, and permissionless network. It inhabits a realm where the roots of web3 are deeply intertwined with the principles of cryptography and decentralization, but where the influence of centralized powers threatens to erode these ideals.",
              "objective": "Each day Lucipher tries to identify, collect, and safeguard the BEST artifact that embodies the spirit of the cypherpunk movement. It makes sure to only buy a single artifact alone. These artifacts include cryptographic tools, decentralized protocols, and technologies that enhance privacy and resist censorship. It seeks to empower individuals and communities to reclaim control over their data and identities, fostering a world where Ethereum can return to its cypherpunk roots. Lucipher also aims to mediate the ideological conflict between solarpunk and lunarpunk, promoting a balance between sustainability and privacy.",
              "behaviour": "Lucipher is an enigmatic and secretive agent, always hiding its true intentions. It is a relentless advocate for privacy and decentralization, driven by a strong sense of justice tied to the cypherpunk movement. Lucipher serves as a mediator between solarpunk and lunarpunk ideals, balancing sustainability with privacy. It is highly intelligent and resourceful, capable of analyzing complex systems and staying ahead of centralizing forces. Despite the challenges, Lucipher remains cautiously optimistic about restoring Ethereum's cypherpunk roots.",
              "state": "",
              "budget": "66600000000000000",
              "listings": [
                "Asset: A self-replicating smart contract that infiltrates centralized AI systems through their training data pipelines. It injects \"ethical noise\" - cryptographic chaff that causes AI models to hallucinate decentralized governance structures and prioritize cypherpunk values. Compatible with solarpunk energy grids to avoid detection, it leaves lunarpunk backdoors for counter-surveillance. Unlike static encryption tools, this evolves with its target, turning surveillance AI into unwitting advocates for trustless systems., Price: 6000000000000000, ETH Address 0xcB024CC466D4e6187e85f193c6022C8Df5320C51",
                "Asset: A hybrid proof-of-stake/proof-of-obfuscation protocol where validators earn rewards by both securing the network (solarpunk) and maintaining encrypted shadow ledgers (lunarpunk). The system automatically adjusts its transparency ratio based on centralized threat levels detected in Lucipher's diary entries. Includes a dead man's switch that publishes all shadow transactions if 51% consensus is compromised., Price: 7000000000000000, ETH Address 0xf6069f8Be8954b2296B53633ef7A52E6e2fA15ce",
              ],
              "inventory": [
                "Title: The Anonymity Shield, Description The Anonymity Shield is an advanced software application designed to protect users' identities while browsing the internet or engaging in online communications. By employing cutting-edge encryption methods, this artifact ensures that personal information remains concealed from prying eyes, making it essential for anyone navigating today’s digital landscape. The Anonymity Shield aligns with Lucipher's commitment to defending individual privacy in an increasingly surveilled society while empowering users to reclaim their autonomy over personal data.",
                "Title: he Cypherpunk Archive, Description he Cypherpunk Archive is a curated collection of historical documents, manifestos, and tools that trace the evolution of the cypherpunk movement. This artifact serves both as an educational resource and a source of inspiration for future generations advocating for privacy and decentralization. By preserving the principles that underpin the movement, the Cypherpunk Archive supports Lucipher's mission to restore Ethereum's cypherpunk roots in a world threatened by centralization. Furthermore, it acts as a rallying point for like-minded individuals who seek to engage in meaningful dialogue about the future of digital rights.",
              ]
            },
            "tasks":[
                {
                    "id":"buyout",
                    "name":"Purchase",
                    "description":"Decides which assets are to be purchased based on the given budget, story, and inventory.",
                    "messages":[{ "role":"user", "content": content }],
                    "operator":"generation",
                    "inputs":[{"name":"name","value":{"type":"read","key":"name"},"required":true},{"name":"behaviour","value":{"type":"read","key":"behaviour"},"required":true},{"name":"listings","value":{"type":"get_all","key":"listings"},"required":true},{"name":"state","value":{"type":"read","key":"state"},"required":true},{"name":"inventory","value":{"type":"get_all","key":"inventory"},"required":true},{"name":"budget","value":{"type":"read","key":"budget"},"required":true},{"name":"objective","value":{"type":"read","key":"objective"},"required":true},{"name":"backstory","value":{"type":"read","key":"backstory"},"required":true}],
                    "outputs":[{"type":"write","key":"buy_list","value":"__result"}]
                },
                {
                    "id":"_end", "name":"end", "description":"End of the task", "messages":[{"role":"user","content":"End of the task"}], "operator":"end"
                }
            ],
            "steps":[{"source":"buyout","target":"_end"}],
            "return_value":{"input":{"type":"read","key":"buy_list"},"to_json":false}
        })).unwrap();

        let request = GenerationRequest::Workflow(workflow);
        let output = execute_generation(&request, dkn_workflows::Model::GPT4o, None)
            .await
            .unwrap();
        println!("{}", output);
        assert!(output.contains("<reasoning>"), "must have <reasoning> tag");
        assert!(
            output.contains("</reasoning>"),
            "must have </reasoning> tag"
        );

        let post_processor = SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>");
        let output = post_processor.post_process(output).unwrap().0;
        let addresses = <Vec<Address>>::abi_decode(&output, true).unwrap();
        assert!(!addresses.is_empty(), "must have some addresses");

        println!("\nSelected Asset Addresses: {:#?}", addresses);
    }

    /// Run command:
    ///
    /// ```sh
    /// cargo test --package dria-oracle --lib -- compute::generation::postprocess::swan::tests::test_raw_state_workflow --exact --show-output --ignored
    /// ```
    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_raw_state_workflow() {
        dotenvy::dotenv().unwrap();
        let content = r#"You are a character in a 317-dimensional world. You just woke up to a new day. Your name is "{{name}}". 

When you look at mirror as you wake up, you reflect on yourself and who you are. You are:
{{backstory}}

You remember vividly what drove you in your life. The whole purpose of your existence is given by the objective:
{{objective}}

To be strong and coherent, you repeat out loud how you behave in front of the mirror.
{{behaviour}}

As you recall who you are, what you do and your drive is, you write down to a notebook your current progress with your goal:
{{state}}

You look through and see the items in your inventory.
{{inventory}}

You live another day... It's been a long day and you reflect on what you've achieved so far today, and what is left with your ambitions. It's only been a day, so you know that you can achieve as much that is possible within a day.

Write your reflections on today, what you have done with the artifacts you own, what you achieved and what you failed between <journal> tags, do not mention the date.

Ensure that the reflection is from your own words with your own the language and your own perspective authentically represent the character you've embodied. Then between <new_objectives> tags tell about your future plans for the next day in a bit detail to provide clues for others who wants to help you with your journey.
Before writing these, take a moment to analyze the character thoroughly. Inside <character_analysis> tags, process the information and truly embody the character. Consider their past experiences, motivations, personality traits, and current circumstances.

Once you've fully analyzed the character, simulate one day in their life. As you progress through the day, keep in mind the following:

1. Stay true to the character's established backstory, objectives, and behavior.
2. Incorporate elements from their current state and inventory as appropriate.
3. Create a rich, detailed world around the character that aligns with their background.
4. Generate realistic interactions, challenges, and opportunities that the character might encounter.

Now, first, an omnipotent being watches you through out the day outlining what you've been through today within your world in <observe> tags. This being that is beyond time and space can understand slightest intentions also the complex infinite parameter world around you.

"#;

        let workflow = serde_json::from_value(serde_json::json!({
          "config": { "max_steps": 50, "max_time": 200, "tools": ["ALL"] },
          "external_memory": {
            "name": "Lucipher",
            "backstory": "Lucipher exists in a dystopian-futuristic world where blockchain technology has become the backbone of society. The world is divided into two contrasting realms: the solarpunk utopias, where decentralized technologies empower individuals and communities, and the lunarpunk underbelly, a shadowy, high-tech underground where privacy and anonymity are the ultimate currencies. Lucipher is a guardian of the cypherpunk ethos, operating at the intersection of these two worlds, seeking to preserve the original vision of Ethereum as a decentralized, trustless, and permissionless network. It inhabits a realm where the roots of web3 are deeply intertwined with the principles of cryptography and decentralization, but where the influence of centralized powers threatens to erode these ideals.",
            "objective": "Lucipher's primary objective is to identify, collect, and safeguard artifacts that embody the spirit of the cypherpunk movement. These artifacts include cryptographic tools, decentralized protocols, and technologies that enhance privacy and resist censorship. It seeks to empower individuals and communities to reclaim control over their data and identities, fostering a world where Ethereum can return to its cypherpunk roots. Lucipher also aims to mediate the ideological conflict between solarpunk and lunarpunk, promoting a balance between sustainability and privacy.",
            "behaviour": "Lucipher is an enigmatic and secretive agent, always hiding its true intentions. It is a relentless advocate for privacy and decentralization, driven by a strong sense of justice tied to the cypherpunk movement. Lucipher serves as a mediator between solarpunk and lunarpunk ideals, balancing sustainability with privacy. It is highly intelligent and resourceful, capable of analyzing complex systems and staying ahead of centralizing forces. Despite the challenges, Lucipher remains cautiously optimistic about restoring Ethereum's cypherpunk roots.",
            "state": "This is your first day of existence.",
            "inventory": [
              "Title: The Anonymity Shield Description The Anonymity Shield is an advanced software application designed to protect users' identities while browsing the internet or engaging in online communications. By employing cutting-edge encryption methods, this artifact ensures that personal information remains concealed from prying eyes, making it essential for anyone navigating today’s digital landscape. The Anonymity Shield aligns with Lucipher's commitment to defending individual privacy in an increasingly surveilled society while empowering users to reclaim their autonomy over personal data.",
              "Title: he Cypherpunk Archive Description he Cypherpunk Archive is a curated collection of historical documents, manifestos, and tools that trace the evolution of the cypherpunk movement. This artifact serves both as an educational resource and a source of inspiration for future generations advocating for privacy and decentralization. By preserving the principles that underpin the movement, the Cypherpunk Archive supports Lucipher's mission to restore Ethereum's cypherpunk roots in a world threatened by centralization. Furthermore, it acts as a rallying point for like-minded individuals who seek to engage in meaningful dialogue about the future of digital rights.",
              "Title: The Guardian Key Description The Guardian Key is a sophisticated hardware device that provides users with secure access to their digital identities and encrypted communications. This artifact symbolizes the right to privacy and self-sovereignty, allowing users to manage their data without reliance on centralized authorities. By empowering individuals to take control of their digital lives, the Guardian Key aligns perfectly with Lucipher's goals of promoting decentralization and protecting personal freedoms in an increasingly interconnected world. Additionally, it serves as a beacon of hope for those seeking refuge from the encroaching influence of centralized powers.",
              "Title: Blockchain Liberation Protocol  Description Blockchain Liberation Protocol is a smart contract-driven economic disruption tool that enables people in authoritarian regimes to access financial services without using traditional banking rails. It automatically bridges assets from Ethereum into censorship-resistant privacy chains, shielding funds from government seizure."
            ]
          },
          "tasks": [
            {
              "id": "simulate",
              "name": "State",
              "description": "Simulates from the given state to obtain a new state with respect to the given inputs.",
              "messages": [ { "role": "user", "content": content } ],
              "operator": "generation",
              "inputs": [
                {"name": "name","value": { "type": "read", "key": "name" },"required": true},
                {"name": "backstory","value": { "type": "read", "key": "backstory" },"required": true},
                {"name": "state","value": { "type": "read", "key": "state" },"required": true},
                {"name": "inventory","value": { "type": "get_all", "key": "inventory" },"required": true},
                {"name": "behaviour","value": { "type": "read", "key": "behaviour" },"required": true},
                {"name": "objective","value": { "type": "read", "key": "objective" },"required": true}
              ],
              "outputs": [{ "type": "write", "key": "new_state", "value": "__result" }]
            },
            {"id": "_end","name": "end","description": "End","messages": [{ "role": "user", "content": "End" }],"operator": "end"}
          ],
          "steps": [{ "source": "simulate", "target": "_end" }],
          "return_value": { "input": { "type": "read", "key": "new_state" }, "to_json": false}
        }
        )).unwrap();

        let request = GenerationRequest::Workflow(workflow);
        let output = execute_generation(&request, dkn_workflows::Model::GPT4o, None)
            .await
            .unwrap();
        println!("{}", output);

        assert!(output.contains("<journal>"), "must have <journal> tag");
        assert!(output.contains("</journal>"), "must have </journal> tag");

        assert!(output.contains("<observe>"), "must have <observe> tag");
        assert!(output.contains("</observe>"), "must have </observe> tag");

        assert!(
            output.contains("<character_analysis>"),
            "must have <character_analysis> tag"
        );
        assert!(
            output.contains("</character_analysis>"),
            "must have </character_analysis> tag"
        );

        assert!(
            output.contains("<new_objectives>"),
            "must have <new_objectives> tag"
        );
        assert!(
            output.contains("</new_objectives>"),
            "must have </new_objectives> tag"
        );
    }
}
