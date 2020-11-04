#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod raffle {
    #[cfg(not(feature = "ink-as-dependency"))]
    use ink_storage::collections::HashMap;
    use ink_storage::collections::Vec;

    pub type User = AccountId;

    const MINIMUM_TOKEN: Balance = 10000000000000;
    const MAXIMUM_TOKEN: Balance = 100000000000000;

    // Using u32 because Raffle winners is vec where calling len returns u32
    const WINNERS_COUNT: u32 = 2;
    const PLAYER_REQUIRED_TO_START: u32 = 5;

    // Minimum Time for countdown which is 15 min = 15 * 60 * 1000 ms .
    // for testing i use 10000 ms or 10 sec
    const COUNTDOWN_MINIMUM: u64 = 900_000;

    #[ink(storage)]
    pub struct Charity {
        // User can play only once and added to pool with one submission. HashMap is perfect fit here.
        // users_vec so that we can choose it easily when we draw it
        users: HashMap<User, Balance>,
        users_vec: Vec<User>,
        // The collected money is sent  to predefined address. Which is the one who calls the contract first time
        collector: AccountId,
        // Total amount collected till now so we don't have to iterate over users to know it.
        // Just for optimization
        amount_collected: Balance,
        // Winners List
        winners: Vec<User>,
        countdown: Option<Timestamp>,
    }

    #[ink(event)]
    pub struct Entry {
        user: AccountId,
    }

    #[ink(event)]
    pub struct WinnerChoosen {
        user: AccountId,
    }

    #[ink(event)]
    pub struct CountDownStarted {
        user: AccountId,
        timestamp: Timestamp,
    }

    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    pub enum Error {
        // The event is already completed so there is no need to do entry
        Completed,
        InvalidEntryAmount,
        AlreadyGotEntry,

        CountDownNotStartedYet,

        // There are few users so draw cannot start
        LessUsers,
    }

    pub type Re<T> = core::result::Result<T, Error>;
    pub use Error::*;

    impl Charity {
        #[ink(constructor)]
        pub fn new(collector: AccountId) -> Self {
            Self {
                collector,
                users: HashMap::new(),
                users_vec: Vec::new(),
                winners: Vec::new(), // I don't know if capacity is avail :D
                countdown: None,
                amount_collected: 0,
            }
        }

        #[ink(message)]
        pub fn get_collector_id(&self) -> User {
            self.collector
        }

        #[ink(message)]
        pub fn current_user_count(&self) -> u32 {
            self.users_vec.len()
        }

        #[ink(message)]
        pub fn get_amount_collected(&self) -> Balance {
            self.amount_collected
        }

        #[ink(message)]
        pub fn winners_count(&self) -> u32 {
            ink_env::debug_println("pleb: winner_count start");
            let len = self.winners.len();
            ink_env::debug_println("pleb: winner_count  completed");
            len
        }

        #[ink(message)]
        pub fn is_completed(&self) -> bool {
            ink_env::debug_println("pleb: is_completed Start");
            let len = self.winners_count();
            ink_env::debug_println("pleb: is_completed Completed");

            len == WINNERS_COUNT
        }

        #[ink(message)]
        pub fn winners_address(&self) -> (Option<User>, Option<User>) {
            (
                self.winners.first().map(|v| v.clone()),
                self.winners.last().map(|v| v.clone()),
            )
        }

        #[ink(message, payable)]
        pub fn get_entry(&mut self) -> Re<()> {
            ink_env::debug_println("pleb: Get Entry Func");

            if self.is_completed() {
                ink_env::debug_println("pleb: Inside Completed");
                return Err(Completed);
            }

            ink_env::debug_println("pleb: Caller");

            let caller = self.env().caller();

            ink_env::debug_println("pleb: Amount");
            let amount = self.env().transferred_balance();

            ink_env::debug_println("pleb: Check Amount is sufficient");
            // Verify First Approach
            // First we check whether the amount is correct or not
            if amount > MAXIMUM_TOKEN || amount < MINIMUM_TOKEN {
                ink_env::debug_println("pleb: Inside Amount");
                return Err(InvalidEntryAmount);
            }

            // Second we check if the participant has already got the entry
            ink_env::debug_println("pleb: Outside User Contains Key");
            if self.users.contains_key(&caller) {
                ink_env::debug_println("pleb: Inside User Contains Key");
                return Err(AlreadyGotEntry);
            }

            ink_env::debug_println("pleb: insert user");
            // Mutations
            self.users.insert(caller, amount);
            self.users_vec.push(caller);
            self.amount_collected += amount;

            ink_env::debug_println("pleb: Event");
            self.env().emit_event(Entry { user: caller });

            ink_env::debug_println("pleb: outside countdown");
            if self.countdown.is_none() && self.current_user_count() >= PLAYER_REQUIRED_TO_START {
                ink_env::debug_println("pleb: inside countdown");
                let timestamp = Self::env().block_timestamp();
                self.countdown = Some(timestamp);

                ink_env::debug_println("pleb: emitting countdown event");
                self.env().emit_event(CountDownStarted {
                    timestamp,
                    user: caller,
                });
            }
            ink_env::debug_println("pleb: completed");
            Ok(())
        }

        #[ink(message)]
        pub fn draw(&mut self) -> Re<()> {
            if self.countdown.is_some()
                && Self::env().block_timestamp() - self.countdown.unwrap() < COUNTDOWN_MINIMUM
            {
                return Err(CountDownNotStartedYet);
            }

            if self.is_completed() {
                return Err(Completed);
            }

            // Cache it so we can resue it again
            let user_count = self.current_user_count();

            if user_count < PLAYER_REQUIRED_TO_START {
                return Err(LessUsers);
            }

            let choosed_winner = Self::get_random_number() % user_count;
            let winner = self.users_vec[choosed_winner];
            self.winners.push(winner);
            self.env().emit_event(WinnerChoosen { user: winner });

            let last_player = self.users_vec.pop().unwrap();
            let _ = self.users_vec.set(choosed_winner, last_player);

            Ok(())
        }

        fn get_random_number() -> u32 {
            let seed: [u8; 8] = [1, 1, 1, 1, 1, 1, 1, 1];
            let random_hash = Self::env().random(&seed);
            Self::as_u32_be(&random_hash.as_ref())
        }

        fn as_u32_be(arr: &[u8]) -> u32 {
            ((arr[0] as u32) << 24)
                + ((arr[1] as u32) << 16)
                + ((arr[2] as u32) << 8)
                + ((arr[3] as u32) << 0)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ink_lang as ink;

        #[ink::test]
        fn it_works() {
            let mut charity = Charity::new(default_accounts().alice);
            set_next_caller(default_accounts().bob, Some(MINIMUM_TOKEN + 100));
            let _ = charity.get_entry();
            set_next_caller(default_accounts().charlie, Some(MINIMUM_TOKEN + 100));
            let _ = charity.get_entry();
            set_next_caller(default_accounts().frank, Some(MINIMUM_TOKEN + 100));
            let _ = charity.get_entry();
            set_next_caller(default_accounts().eve, Some(MINIMUM_TOKEN + 100));
            let _ = charity.get_entry();

            // test same user aren't added
            set_next_caller(default_accounts().eve, Some(MINIMUM_TOKEN + 100));
            let _ = charity.get_entry();

            assert_eq!(charity.current_user_count(), 4);

            set_next_caller(default_accounts().eve, Some(MINIMUM_TOKEN + 100));
            let _ = charity.draw();
            let _ = charity.draw();
            assert_eq!(charity.winners_count(), 2);

            assert_eq!(charity.current_user_count(), 2);
        }

        fn default_accounts() -> ink_env::test::DefaultAccounts<ink_env::DefaultEnvironment> {
            ink_env::test::default_accounts::<ink_env::DefaultEnvironment>()
                .expect("off-chain environment should have been initialized already")
        }

        fn set_next_caller(caller: AccountId, endowment: Option<Balance>) {
            ink_env::test::push_execution_context::<ink_env::DefaultEnvironment>(
                caller,
                AccountId::from([0x1; 32]),
                20000000000,
                endowment.unwrap_or(MINIMUM_TOKEN),
                ink_env::test::CallData::new(ink_env::call::Selector::new([0x00; 4])),
            )
        }
    }
}
