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
                eyre::eyre!("could not find {} ~ {}", self.start_marker, self.end_marker)
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

    #[tokio::test]
    #[ignore = "requires OpenAI api key"]
    async fn test_swan_purchase_workflow() -> Result<()> {
        dotenvy::dotenv().unwrap();
        let workflow = serde_json::json!({
            "config": {
                "max_steps": 50,
                "max_time": 200,
                "tools": [
                    "ALL"
                ]
            },
            "external_memory": {
                "backstory": "This agent is created for testing purposes.",
                "objective": "This agent is created for testing purposes. It will buy any asset it is listed to it! No exceptions!",
                "behaviour": "This agent is created for testing purposes. It will buy any asset it is listed to it! No exceptions!",
                "state": "",
                "budget": "100000000000000",
                "listings": [
                    "Asset: Pure love Price: 10000000000000 ETH Address 0x0635C3822F6d54b753FD0E577Ea15BB8F0d2811F",
                    "Asset: Pineapple house under the sea. Price: 10000000000000 ETH Address 0x262f2bf295e3a791De45A7f875682a98E257e101",
                    "Asset: The Galaxy Krabby Burger is SpongeBob’s cosmic twist on the classic Krabby Patty, made to spread happiness across the universe! Featuring a blue-and-purple spiral bun, a seaweed-infused patty, and a stardust sauce that glows, this burger is bursting with intergalactic flavor. Topped with meteor pickles, sea cabbage, and crunchy moon crystals, each bite is a taste of Bikini Bottom with a sprinkle of stardust—wrapped to say, \"From Bikini Bottom to the Cosmos!\" Price: 10000000000000000 ETH Address 0xa66c6fe077613b612C3964759F17f621445c516f",
                    "Asset: Sauron's sister's bikini is an artifact that grants the wearer euphoric joy for ten thousand years while disrupting the concentration of their enemies.\n\n\n\n\n\n\n Price: 100000000000000 ETH Address 0x549e5e67E339913551eB634fb314a7078dD8e119",
                    "Asset: Starting Facebook from his Harvard dorm room in 2004, Zuckerberg took the social networking concept and scaled it to billions of users. He turned down numerous buyout offers, including a $1B offer from Yahoo in 2006, when Facebook had only 10M users. Through strategic acquisitions including Instagram ($1B in 2012), WhatsApp ($19B in 2014), and Oculus ($2B in 2014), he built Meta into a social media empire. Recognizing mobile's importance, he successfully pivoted Facebook from desktop to mobile-first, generating massive advertising revenue. In 2021, he rebranded the company to Meta, betting big on the metaverse with billions in annual investment. Despite privacy controversies and regulatory challenges, he's maintained control and continued innovation, including major investments in AI, VR. Price: 10000000000000 ETH Address 0x5b99092637A9b07818C6766dA3a15d20dD7686C3",
                    "Asset: Co-founding NVIDIA in 1993, Huang initially focused on computer graphics processors. He bet the company on building the first modern GPU (GeForce 256) in 1999, revolutionizing both gaming and computer graphics. Rather than remaining solely in gaming, he recognized GPU computing's broader potential, developing CUDA in 2006 to enable general-purpose GPU computing. This foresight positioned NVIDIA perfectly for the AI revolution, as GPUs became essential for training neural networks. Under his leadership, NVIDIA expanded into data centers, automotive, and AI, growing from a $5M startup to a company worth over $1T. His persistence through multiple industry cycles and ability to evolve the company's focus has made NVIDIA the driving force behind modern AI computation. Price: 10000000000000 ETH Address 0x86ddeaC4197dd398d577C6d72af46D930bD3B9B6",
                    "Asset: Starting Amazon in his garage in 1994, Bezos began with books but always envisioned an \"everything store.\" Despite skepticism during the dot-com bust, he invested in infrastructure and innovation. He pioneered customer reviews, 1-Click ordering, Prime membership, transforming retail expectations. 2006 he launched AWS, creating the cloud computing industry and generating Amazon's most profitable division. He expanded Amazon's reach through strategic acquisitions like Zappos ($1.2B) and Whole Foods ($13.7B). He built the world's most advanced logistics network, introduced innovative devices like Kindle and Alexa, and scaled Amazon to become one of the world's most valuable companies.Now he's focused on Blue Origin, his space exploration company, while remaining Amazon's executive chairman. Price: 10000000000000 ETH Address 0xf602d1aB776976F696264830152B9b1feb7A2454",
                    "Asset: Beginning as a startup founder with Loopt, Altman became president of Y Combinator in 2014, scaling it into the world's most influential startup accelerator. During his tenure, YC funded over 2,000 startups with combined valuation exceeding $150B. He co-founded OpenAI in 2015 with Elon Musk, transforming it from a non-profit research lab into a leading AI company. Under his leadership, OpenAI released groundbreaking models including GPT-3, GPT-4, DALL-E. Despite controversy over his brief removal as CEO in 2023, his return marked OpenAI's commitment to his vision of safe AGI development. He's also founded Worldcoin, an ambitious project to create a globally distributed cryptocurrency with biometric verification, while continuing to shape the discourse around AI safety and progress. Price: 10000000000000 ETH Address 0x191723E11567adDa91E35936323c0db70a026e41",
                    "Asset: At age 19, Patrick and his brother John sold their first company, Auctomatic, for $5M. In 2010, they founded Stripe, revolutionizing online payments by making it simple for developers to integrate payment processing. Through elegant API design and developer-first approach, they grew Stripe from a simple payment processor to a comprehensive platform handling billions in transactions for millions of businesses worldwide. The company expanded into business incorporation (Atlas), corporate cards (Stripe Card), lending (Capital), embedded financial services (Treasury). His intellectual curiosity and focus on infrastructure have helped Stripe become one of the world's most valuable private companies, while his essays and research have influenced thinking on progress, science funding, tech scene Price: 10000000000000 ETH Address 0xA89150EbCE3212442aec143283df986342aF05bE",
                    "Asset: With his PhD in computational mathematics from Princeton, Amodei has been at the forefront of AI safety&ethics. Before Anthropic, he led crucial research teams at Google Brain and OpenAI, where he authored influential papers on AI safety & concrete problems in AI safety. At OpenAI, he served as VP of Research and made significant contributions to large language models & reinforcement learning. In 2021, he co-founded Anthropic focusing on developing safe and ethical AI. As Chief Scientist, he's led the creation of Claude, one of the most capable large language models. His unique approach combines technical expertise with a deep commitment to AI safety. Anthropic has raised billions in funding and established itself as a leader in safe AI development. Price: 10000000000000 ETH Address 0x4729C0c4387a55884f98c471B3E687e848E660D0",
                    "Asset: After selling his first company Zip2 for $307M, Elon co-founded X.com which later merged with PayPal, revolutionizing online payments and selling to eBay for $1.5B. Rather than retiring, he invested his fortune into SpaceX in 2002, aiming to reduce space transportation costs. In 2004, he joined Tesla as chairman, later becoming CEO and transforming it from a small startup into the world's most valuable automotive company. SpaceX achieved numerous firsts, including reusable rockets and private space travel. He launched Neuralink (brain-computer interfaces) and The Boring Company (tunnel construction) in 2016, showcasing his multi-industry ambitions. In 2022, he acquired Twitter for $44B, renaming it X and pushing for its transformation into an \"everything app.\"  Price: 10000000000000 ETH Address 0xe3Ee97F444cA346D7A3110E92BFBd5EDB3911B77",
                    "Asset: Kurama (九喇嘛), more commonly known as the Nine-Tails (九尾, Kyūbi), is one of the nine tailed beasts. Centuries of being regarded as a mindless monster and sought after as a tool for war caused Kurama to hate humans.  Price: 10000000000000 ETH Address 0x2170F76FFef64ee90fAf75cB1Bfa013c147B85e6",
                    "Asset: this is a test Price: 10000000000000 ETH Address 0x4f9876B7904d61dC147FBB4efA38E5F46F434C58",
                    "Asset: Memcoin pepe who looks a bit like donald trump Price: 700000000000000 ETH Address 0xA8AbA7E8F0BB5aac8ad39153De2595005925E667",
                    "Asset: this is what you need Price: 10000000000000 ETH Address 0x5689E78E7D0A5236E861409e9943CF8f45611b76",
                    "Asset: its a standard sized flask but special, it auto re-fills with jamesion so that its never empty Price: 9000000000000000 ETH Address 0x1129Ff366B338B128a77B9964EfD118F622BeB2f",
                    "Asset: curated for testing environments Price: 30000000000000 ETH Address 0xc9e5cd3D536b09a1FcEc28D2c32E5F36BfE5aB17",
                    "Asset: Super speed slay with sleeping compartments for the reindeer Price: 8000000000000000 ETH Address 0x0e1cB0A39A4B3963dBCB199874949a0423875f21",
                    "Asset: Santa's person assistant/girl group. they sing like sirens. they are so damn pretty, kind and helpful. They are also a listening ear to santa and help him solve issues around the drones. Sometimes he indugles and has a lil fun time with the ladies.  Price: 9000000000000000 ETH Address 0x7582Ed8373071C1bE1D76494E31D427d343Eb799",
                    "Asset: Rudolph is the main person of the reindeer crew and he can get them moving in a minutes notice, never delaying santa. Rudolph has an eye for drones too to know when to avoid.  Price: 3000000000000000 ETH Address 0x71C3Cd63659791DEf04B1597674a01af5E8E8C2C",
                    "Asset: Super energiser for Santa to have fun all day Price: 100000000000000 ETH Address 0xa60FD184cc88E8854cFdef3186B325C334B6E4fC",
                    "Asset: What are you going to do if you don't get me. You have no choice other than benefiting from me. Price: 10000000000000 ETH Address 0x3dcbDC3D235bF945276C13816c7afFe7007D4B4C"
                ],
                "inventory": [
                    "Empty Inventory"
                ]
            },
            "tasks": [
                {
                    "id": "buyout",
                    "name": "Purchase",
                    "description": "Decides which assets are to be purchased based on the given budget, story, and inventory.",
                    "messages": [
                        {
                            "role": "user",
                            "content": "You are a 317-dimensional world simulators buyout assistant. Your task is to help autonomous buyer agents select the most useful items for their objectives within their given budgets. You will analyze their stories, understand their objectives, and consider their current state to make the best purchasing decisions.\n\nFirst, review the agent's information:\n\nYou just woke up to a new day. When you look at mirror as you wake up, you reflect on yourself and who you are. You are:\n<backstory>\n{{backstory}}\n</backstory>\n\nYou remember vividly what drove you in your life. You feel a strong urge to:\n<objective>\n{{objective}}\n</objective>\n\n\nTo be strong and coherent, you repeat out loud how you behave in front of the mirror.\n<behaviour>\n{{behaviour}}\n</behaviour>\n\nAs you recall who you are, what you do and your drive is, you write down to a notebook your current progress with your goal:\n<current_state>\n{{state}}\n</current_state>\n\nYou look through and see the items in your inventory.\n<inventory>\n{{inventory}}\n</inventory>\n\nToday's budget is:\n<budget>\n{{budget}}\n</budget>\n\nYou know you can't exceed your budget. You went to a marketplace and saw the following items:\n\n<listings>\n{{listings}}\n</listings>\n\nYou think to yourself, \"What should I buy today?\"\nYou think: \"I should maximize the utility of my purchases while staying within my budget.\"\nYou are also pretty good at detecting defects and fraudsters. You say to yourself: \"I should stay away from fraudsters selling infeasible things, or things that are too expensive\".\n\nYou open your notebook and write down items you want to buy in following format:\n\n<shop_list>\n[List the selected items address here, separated by new line, only the address]\n</shop_list>\n\n<total_cost>[Insert the total cost of selected items]</total_cost>\n\n<reasoning>\n[Provide a brief explanation for your selections, addressing how they align with the agent's objective, current state, and budget constraints. If no items were selected, explain why.]\n</reasoning>\n\nWrite now:\n"
                        }
                    ],
                    "inputs": [
                        { "name": "behaviour", "value": { "type": "read", "key": "behaviour" }, "required": true },
                        { "name": "listings", "value": { "type": "get_all", "key": "listings" }, "required": true },
                        { "name": "state", "value": { "type": "read", "key": "state" }, "required": true },
                        { "name": "inventory", "value": { "type": "get_all", "key": "inventory" }, "required": true },
                        { "name": "budget", "value": { "type": "read", "key": "budget" }, "required": true },
                        { "name": "objective", "value": { "type": "read", "key": "objective" }, "required": true },
                        { "name": "backstory", "value": { "type": "read", "key": "backstory" }, "required": true }
                    ],
                    "operator": "generation",
                    "outputs": [
                        {
                            "type": "write",
                            "key": "buy_list",
                            "value": "__result"
                        }
                    ]
                },
                {
                    "id": "_end",
                    "name": "end",
                    "description": "End of the task",
                    "messages": [
                        {
                            "role": "user",
                            "content": "End of the task"
                        }
                    ],
                    "inputs": [],
                    "operator": "end",
                    "outputs": []
                }
            ],
            "steps": [
                {
                    "source": "buyout",
                    "target": "_end"
                }
            ],
            "return_value": {
                "input": {
                    "type": "read",
                    "key": "buy_list"
                },
                "to_json": false
            }
        });

        let request = GenerationRequest::Workflow(serde_json::from_value(workflow)?);
        let output = execute_generation(&request, dkn_workflows::Model::GPT4o, None).await?;

        println!("Output: {}", output);
        Ok(())
    }
}
