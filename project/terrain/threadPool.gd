extends Node

const NUM_WORKERS = 2  # Or adjust based on CPU cores
var tasks = []
var tasks_mutex = Mutex.new()
var workers = []
var libchunk_generator = null
var semaphore = Semaphore.new()  # Add a semaphore for signaling

func _ready():
	# Create a fixed number of threads
	for i in range(NUM_WORKERS):
		var t = Thread.new()
		var ret = t.start(Callable(self, "_worker_loop").bind(i))
		if ret != OK:
			push_error("Failed to start worker thread " + str(i))
		workers.append(t)
	print("ThreadPool: Started %d worker threads" % NUM_WORKERS)

# Called by your terrain code: queue a "chunk generation" job
func enqueue_chunk(cx, cy, CHUNK_SIZE):
	tasks_mutex.lock()
	tasks.append({"cx": cx, "cy": cy, "CHUNK_SIZE": CHUNK_SIZE})
	tasks_mutex.unlock()
	semaphore.post()  # Signal that a task is available
	print("ThreadPool: Enqueued chunk at (%d, %d)" % [cx, cy])

func _worker_loop(worker_id):
	print("ThreadPool: Worker %d started" % worker_id)
	
	while true:
		# Wait for a task to be available
		semaphore.wait()
		
		# Get a task
		tasks_mutex.lock()
		var task = null
		if tasks.size() > 0:
			task = tasks.pop_front()
		tasks_mutex.unlock()
		
		if task:
			print("ThreadPool: Worker %d processing chunk at (%d, %d)" % [worker_id, task.cx, task.cy])
			# Do the heavy chunk generation off-thread:
			var chunk = null
			if libchunk_generator:
				# First generate biome data (lightweight)
				var biome_data = libchunk_generator.generate_biome_data(
					task.cx, task.cy, task.CHUNK_SIZE)
				
				# Then generate the chunk with that data
				chunk = libchunk_generator.generate_chunk_with_biome_data(
					task.cx, task.cy, biome_data)
				
				# Defer final scene add to the main thread
				call_deferred("_finalize_chunk", chunk, task.cx, task.cy)
			else:
				push_error("ThreadPool: Worker %d has no libchunk_generator reference" % worker_id)

func _finalize_chunk(chunk, cx, cy):
	if chunk:
		# Add the chunk to the scene
		get_parent().add_child(chunk)
		# Update loaded chunks
		get_parent().loaded_chunks[Vector2i(cx, cy)] = true
		print("ThreadPool: ✅ Chunk at (%d, %d) finalized" % [cx, cy])
	else:
		push_error("ThreadPool: Failed to finalize chunk at (%d, %d) - chunk is null" % [cx, cy])
		
func _exit_tree():
	print("ThreadPool: Cleaning up threads")
	# Clean up threads
	for i in range(workers.size()):
		# Post to semaphore to wake up any waiting threads
		semaphore.post()
	
	# Wait for all threads to finish
	for i in range(workers.size()):
		workers[i].wait_to_finish()
	print("ThreadPool: All threads cleaned up")