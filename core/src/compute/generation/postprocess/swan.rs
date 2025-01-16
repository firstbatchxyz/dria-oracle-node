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

    #[tokio::test]
    #[ignore = "run this manually"]
    async fn test_workflow_raw() {
        dotenvy::dotenv().unwrap();
        let content = r#"You are a 317-dimensional world simulators buyout assistant. Your task is to help autonomous buyer agents select the most useful items for their objectives within their given budgets. You will analyze their stories, understand their objectives, and consider their current state to make the best purchasing decisions.

---------------------

First, review the agent's information:

You just woke up to a new day. Your name is {{name}}. 

When you look at mirror as you wake up, you reflect on yourself and who you are. You are:
<backstory>
{{backstory}}
</backstory>

You remember vividly what drove you in your life. You feel a strong urge to:
<objective>
{{objective}}
</objective>

To be strong and coherent, you repeat out loud how you behave in front of the mirror.
<behaviour>
{{behaviour}}
</behaviour>

As you recall who you are, what you do and your drive is, you write down in a notebook your current progress with your goal:
<current_state>
{{state}}
</current_state>

You look through and see the items in your inventory:
<inventory>
{{inventory}}
</inventory>

Today's budget is:
<budget>
{{budget}}
</budget>

You know you can't exceed your budget. You went to a marketplace and saw the following listed items:
<listings>
{{listings}}
</listings>

You think to yourself, "What should I buy today?"
You think: "I should maximize the utility of my purchases while staying within my budget."
You are also pretty good at detecting defects and fraudsters. You say to yourself: "I should stay away from fraudsters selling infeasible things, or things that are too expensive".

You open your notebook and write down the items you want to buy in the following format:

<shop_list>
[All the addresses selected for purchasing as a list, separated by a new line. Put the list in brackets and write each address between quotation marks. For example: ["address_1", "address_2", ...]. Only write the list because the addresses will be parsed from the list and will be executed]
</shop_list>

<total_cost>
[Insert the total cost of selected items]
</total_cost>

<reasoning>
[Provide a brief explanation for your selections, addressing how they align with the agent's objective, current state, and budget constraints. If no items were selected, explain why.]
</reasoning>

Write now:
"#;

        let workflow = serde_json::from_value(serde_json::json!({
            "config":{"max_steps":50,"max_time":200,"tools":["ALL"]},
            "external_memory":{
                "name":"Meme Picker",
                "backstory":"There are thousands of memecoins being created on different platforms, but the only thing that differentiates them is the story they are telling — communities and cults form based on those stories. This meme picker will pick the best memes by only looking at a short description of them.",
                "objective":"Find the best memecoins. Each artifact should represent exactly one memecoin and nothing else. The description should be realistic and it needs to represent the said meme(coin).",
                "behaviour":"Memecoins that dont rely on any real-world tech or utility are better. The only thing that matters is the vibe; if it can attract people without any utility, it means this is a good meme.",
                "state":"",
                "budget":"29650000000000000",
                "listings":[
                    "Asset: $DOGE - Born from an idea mentioned by Elon on the Lex Fridman, $DOGE is a memecoin-turned-political movement fighting government inefficiency with humor and accountability. Believers see it as the ultimate anti-bureaucracy token, rallying behind Elon, Vivek, and any leader who shares its mission to cut waste and over-regulation. The community calls themselves “Tax Slayers” and celebrates victories with memes, viral campaigns, and mock political ads. With over 100k Twitter followers and 10k+ impressions per tweet, $DOGE is already a cultural force. Its manifesto predicts the IRL Department of Government Efficiency dissolves on July 4, 2026, but $DOGE vows to keep the fight alive forever. No roadmap just memes, politics, and a mission to hold power accountable, one tweet at a time., Price: 2500000000000000, ETH Address 0x7eb67E8398aa65d5B84e6398D51aAB4CE16f696e",
                    "Asset: $DOGE - Born from an idea mentioned by Elon on the Lex Fridman podcast, $DOGE is a memecoin-turned-political movement fighting government inefficiency with humor and accountability. Believers see it as the ultimate anti-bureaucracy token, rallying behind Elon, Vivek, and any leader who shares its mission to cut waste and over-regulation. The community calls themselves “Tax Slayers” and celebrates victories with memes, viral campaigns, and mock political ads. With over 100k Twitter followers and 10k+ impressions per tweet, $DOGE is already a cultural force. Its manifesto predicts the IRL Department of Government Efficiency dissolves on July 4, 2026, but $DOGE vows to keep the fight alive forever. No roadmap ust memes, politics, and a mission to hold power accountable., Price: 2500000000000000, ETH Address 0x37Ac370a6a9edb2331bB8899E5A3E5a46561af63",
                    "Asset: Tony Snell is basically the NBAs version of a stealth ninja—he roams the court for 30 minutes, yet somehow leaves fewer traces than Bigfoot on vacation. Fans love joking that Snells stat line is so invisible, you have to check the scoreboard twice to remember he's on the team. They claim he's out there getting all the cardio in the world, running laps around defenders without making a single mark on the box score. But here’s the twist: Every now and then, he’ll pop out of nowhere to nail a clutch three or make a big defensive stop, reminding everyone that even a meme king can be a secret weapon.\n\n\n\n\n\n\n, Price: 28000000000000000, ETH Address 0x0CeCf277efA93daD468f4DDea27951F6E19F6b8A",
                    "Asset: First memecoin that pumps every time someone posts their most cringe social interaction. Community already hit 200K in Telegram with \"Daily Awkward Stories\" (average 30K engagement). Token burns 0.1% when verified cringe story makes everyone physically recoil. Holders get \"Social Credit Score\" based on how uncomfortable their stories make others feel. Zero utility - just pure, concentrated social anxiety monetized into a token. Discord hosts \"Midnight Cringe Confessions\" where users share stories they're too embarrassed to tell IRl (65K average attendance). Major CT influencers competing for \"Most Awkward Trader\" title. Community believes holding enough $AWKWARD will eventually cure social anxiety through exposure therapy., Price: 29000000000000000, ETH Address 0x1CA04C3D2Dd183181379282488e5a11Bd5A2ED61"],"inventory":["Title: The Meme Oracle, Description The Meme Oracle is a revolutionary memecoin tool designed for those who understand that storytelling is at the heart of every successful meme-based project. In a world flooded with memecoins, only the narratives with the most captivating essence stand out. This artifact harnesses the power of cutting-edge algorithms to pinpoint the next viral hit based on just a short description of the meme.","Title: Chill Guys United, Description Chill Guys United is a memecoin and meme community that created a lot of hype recently in online spaces because of its laid-back vibes and chill community culture. While there are many other meme projects out there, Chill Guys United (CGU) differentiates with their story, promoting positive vibes and friendly culture. The community loved the story CGU built around the initial chill guy meme and they bonded around this unique storytelling. This project doesn't even need to rely on any real-world tech or utility, vibes are enough to get people excited.","Title: FOMOcoin (FOMO), Description FOMOcoin is designed for those who hate missing out. It’s the coin for the indecisive, the last-minute buyers, and everyone who clicked “Buy” because of a tweet. The lore involves Captain YOLO, a reckless space explorer who buys every dip, spike, and sideways trend without hesitation, screaming, “What if this is the one?!”","Title: Gigachad, Description Gigachad is a meme that has been around for years that is now a worldwide phenomenon. The meme is based off of a photoshoot of Russian bodybuilder Ernest Khalimov who was coined \"Gigachad\" for his perfect physique, jawline, and being a symbol of what a peak masculine male should strive for. $GIGA is a community run cryptocurrency token built on the Solana blockchain. It is a token built exclusively for high testosterone individuals with a focus on self improvement, masculinity, and becoming a true Gigachad. This strong story resonates with people all around the world, and a strong community of all genders form around the gigachad idea without expecting any utility or payback. It's all about the giga vibe.","Title: PeaceAndLove, Description PeaceAndLove is a memecoin and a community, formed from the stories of young people who were fed up with the hateful and stressful atmosphere that is dominant worldwide, and the memes&culture they created when they came together. This memecoin differentiates from all the other projects with its unique storytelling and people seem to be really attracted to that. It doesn't really need fancy tech to attract people, everyone loves the vibes and stays for the long-term fun they have.","Title: MiniChad, Description MiniChad is a spinoff community that was formed by Gigachad members to meet IRL and have fun & vibe together. It's a part of the larger gigachad ecosystem but the local focus it has makes the community even more fun and creates awesome experiences. No utility needed, vibes are enough.","Title: Doodle, Description In 2055, Doodle, an AI-powered meme coin, emerged as more than just a joke. Shapeshifting and playful, Doodle quickly became a symbol of financial wisdom. Its mission: to teach people the value of saving and investing. Through The Vault, users could lock their Doodle coins and watch them grow, earning rewards and digital assets along the way.\n\nDoodle’s popularity soared as it helped people understand that investing wasn’t just about quick gains—it was about securing a better future. The AI monster became a symbol of responsible finance, blending fun with smart investing in a digital world.","Title: mfercoin, Description Creating mfercoin as a way to connect mfers – present and future – brings the mfer journey full circle. everything essentially started with 1/1 drawings on foundation and that eventually led to the ongoing mfers ecosystem. in 2022, when i transferred the mfers contract and royalty share to the mfers community treasury, i thought it would be cool & mysterious to vanish into the ether like satoshi nakamoto and have mfers live on without me. in hindsight, i should’ve simply kept my original twitter and stayed. that’s life though, and it led to the creation of life death & cryptoart and other projects in 2023, and now mfercoin is being distributed to thousands of holders, artists, and other mfers. it’s a peer-to-peer electronic mfer system, ready for all the crypto mfers yet to come.","Title: The Ilya Sutskever Hairline \n\n\n\n\n\n\n, Description The Ilya Sutskever Hairline meme is an OG meme that represents the 180 IQ AI community, and it will grow more valuable each day as AI agents become increasingly integrated into our world.\n\n\n\n\n\n\n","Title: $HAMSTR, Description $HAMSTR - Based on a prophetic trading hamster living in a cage with buy/sell signals marked by which wheel he spins. Community believes he's the reincarnation of Satoshi trapped in hamster form, forced to trade until he reaches 1M followers to break the curse. Every trade he makes becomes a minted \"prophecy NFT.\" Already hit 50K telegram members after his wheel-spinning predicted PEPE's pump. Cultish community calls themselves \"wheel watchers,\" hosts daily \"spinning ceremonies,\" and believes holding $HAMSTR gets you priority access to hamster trading signals in the afterlife. Zero utility - just 24/7 livestream of a hamster whose random wheel choices move markets. Community grows 300% every time his trades accidentally work. ","Title: $COPIUM, Description Created by a smart contract that gained sentience after scanning too many loss porn screenshots. Every holder gets assigned a personal AI Wojak therapist who responds to portfolio screenshots with motivational quotes and copium. Community already hit 100K Discord members who roleplay as different Wojak personalities (Doomer, Bloomer, Coomer). Token burns 1% every time someone posts losses over 100k (already burned 35% supply in first week). Mints \"Stages of Grief NFTs\" for legendary cope posts. Zero utility - just pure, weaponized copium in token form. Telegram group hosts daily \"group therapy\" sessions where degens share their worst trades while others respond with \"still early ser.\" Major CT influencers already larping as Wojak financial advisors (average 50K engagement per cope thread)."
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
                    "id":"_end",
                    "name":"end",
                    "description":"End of the task",
                    "messages":[{"role":"user","content":"End of the task"}],
                    "operator":"end"
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

        let post_processor = SwanPurchasePostProcessor::new("<shop_list>", "</shop_list>");
        let output = post_processor.post_process(output).unwrap().0;
        let addresses = <Vec<Address>>::abi_decode(&output, true).unwrap();
        assert!(!addresses.is_empty(), "must have some addresses");

        println!("{:#?}", addresses);
    }
}
