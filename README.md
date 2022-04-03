`USAGE:
    payment_engine [OPTIONS] <FILE_PATH>`

### Correctness
Most of the correctness is ensured by the type system and the few checks done at deserialization / conversion steps. A few test check the normal behavior of the program.

### Robustness
Errors are divided into 2 groups, (which allows the library to be split into 2 parts as well)
 - input format errors for wrong input data format
 - errors that don't respect the application logic (missing transaction, wrong state, not enough funds, etc.). 

We can use the -d or --debug flag to get details about the different processing errors.

### Efficiency 
Transaction records are processed on the fly and not stored. Only money operations (deposits and withdrawals) are saved progressively in order to act on the operation history in case of a client claim.
This makes this program quite effcient in terms of
memory usage, but not so much in terms of speed.

As the operations are sorted chronogically in a file, using concurrency is a bit tricky.
If different threads process different files and those files can point to the same transactions or clients,  we don't have the guarantee that transactions are processed in the right chronological order. This is not irrelevant to our program as a withdrawal shouldn't be possible in case of insufficient funds, or, in extreme cases of large gaps between file processing, claims can reference transactions that are still on the queue. If having the order of transactions not always respected is acceptable for business, then the implementation would be to share a Mutex to the account summary and to the transaction register (Mutex because both read and write are necessary in most cases) between threads that take locks sequentially.

One way to take advantage of concurrency for this program would be at the client level instead of the transaction level => Form groups of clients that a thread is reponsible for, then each operation
can go to a specific channel that is unstacked by a specific thread (ie one for clients 1-9999, one for clients 10000-19999, etc.).

This would allow to process a unique file way faster than the current way by sharing the work without risking to mess with the order of operations.
In this case we can split the transaction and account registers and won't need locks to shared structures. 

Obviously in the real world there is a good chance that we use a proper database, some of the locking and data race issues would be handled by Postgres transactions for instance, but this does not handle the chronological order problem.

Another simplified scenario could be that concurrent TCP streams provide files that concern different groups of clients. The processing could then be fully parallelized: one shared database is used by every thread, they each make their corresponding file operations flow from the stream to their handling function, queue for the access to the database, etc. 
