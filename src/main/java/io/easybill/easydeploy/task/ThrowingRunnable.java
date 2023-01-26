package io.easybill.easydeploy.task;

@FunctionalInterface
public interface ThrowingRunnable {

  void run() throws Exception;
}
